/*
Razor SF-data net: (768 -> 768)x2 -> 1 SCReLU trained on Stockfish's public
NNUE training data (test80 binpacks, labeled by deep SF/Leela search ~3500 Elo).
This breaks past the self-play teacher ceiling (~2600). See STATE.md.

Same 768 arch as net4 so SF-data-768 vs selfplay-data-768 is a clean comparison.
*/
use bullet_lib::{
    game::inputs::Chess768,
    nn::optimiser::AdamW,
    trainer::{
        save::SavedFormat,
        schedule::{TrainingSchedule, TrainingSteps, lr, wdl},
        settings::LocalSettings,
    },
    value::{ValueTrainerBuilder, loader},
};

const HIDDEN_SIZE: usize = 768;
const SCALE: i32 = 400;
const QA: i16 = 255;
const QB: i16 = 64;

fn main() {
    let mut trainer = ValueTrainerBuilder::default()
        .dual_perspective()
        .optimiser(AdamW)
        .inputs(Chess768)
        .save_format(&[
            SavedFormat::id("l0w").round().quantise::<i16>(QA),
            SavedFormat::id("l0b").round().quantise::<i16>(QA),
            SavedFormat::id("l1w").round().quantise::<i16>(QB),
            SavedFormat::id("l1b").round().quantise::<i16>(QA * QB),
        ])
        .loss_fn(|output, target| output.sigmoid().squared_error(target))
        .build(|builder, stm_inputs, ntm_inputs| {
            let l0 = builder.new_affine("l0", 768, HIDDEN_SIZE);
            let l1 = builder.new_affine("l1", 2 * HIDDEN_SIZE, 1);
            let stm_hidden = l0.forward(stm_inputs).screlu();
            let ntm_hidden = l0.forward(ntm_inputs).screlu();
            let hidden = stm_hidden.concat(ntm_hidden);
            l1.forward(hidden)
        });

    let schedule = TrainingSchedule {
        net_id: "razorsf".to_string(),
        eval_scale: SCALE as f32,
        steps: TrainingSteps {
            batch_size: 16_384,
            batches_per_superbatch: 6104,
            start_superbatch: 1,
            // SF data is large + high quality — more passes than the 100M self-play nets
            end_superbatch: 80,
        },
        wdl_scheduler: wdl::LinearWDL { start: 0.4, end: 0.8 },
        lr_scheduler: lr::StepLR { start: 0.001, gamma: 0.3, step: 30 },
        save_rate: 20,
    };

    let settings =
        LocalSettings { threads: 4, test_set: None, output_directory: "checkpoints_razorsf", batch_queue_size: 64 };

    // Stockfish public binpack (decompressed). SfBinpackLoader streams games and
    // applies the standard SF training filter: skip early plies, in-check
    // positions, huge scores, and non-quiet/capture best moves.
    let data_loader = {
        use loader::sfbinpack::{MoveType, PieceType, SfBinpackLoader, TrainingDataEntry};
        fn filter(entry: &TrainingDataEntry) -> bool {
            entry.ply >= 16
                && !entry.pos.is_checked(entry.pos.side_to_move())
                && entry.score.unsigned_abs() <= 10000
                && entry.mv.mtype() == MoveType::Normal
                && entry.pos.piece_at(entry.mv.to()).piece_type() == PieceType::None
        }
        SfBinpackLoader::new(r"H:\RazorBot\data\sf\test80-2024-01-jan.binpack", 1024, 4, filter)
    };

    trainer.run(&schedule, &settings, &data_loader);
}
