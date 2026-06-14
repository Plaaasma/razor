/*
Razor SF net 2: BIGGER (768 -> 1024)x2 -> 1 SCReLU on 4 months of Stockfish
test80-2024 data (jan-apr, ~1.2B+ positions before filter). 1024 width is
justified now that data is abundant + high quality (net3's starvation lesson).
Toward M1. See STATE.md v0.8.0 plan.
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

const HIDDEN_SIZE: usize = 1024;
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
        net_id: "razorsf2".to_string(),
        eval_scale: SCALE as f32,
        steps: TrainingSteps {
            batch_size: 16_384,
            batches_per_superbatch: 6104,
            start_superbatch: 1,
            // 4 months of data + bigger net: more passes
            end_superbatch: 100,
        },
        wdl_scheduler: wdl::LinearWDL { start: 0.4, end: 0.8 },
        lr_scheduler: lr::StepLR { start: 0.001, gamma: 0.3, step: 40 },
        save_rate: 20,
    };

    let settings =
        LocalSettings { threads: 6, test_set: None, output_directory: "checkpoints_razorsf2", batch_queue_size: 64 };

    // all 4 Stockfish test80-2024 months, concatenated by the loader
    let data_loader = {
        use loader::sfbinpack::{MoveType, PieceType, SfBinpackLoader, TrainingDataEntry};
        fn filter(entry: &TrainingDataEntry) -> bool {
            entry.ply >= 16
                && !entry.pos.is_checked(entry.pos.side_to_move())
                && entry.score.unsigned_abs() <= 10000
                && entry.mv.mtype() == MoveType::Normal
                && entry.pos.piece_at(entry.mv.to()).piece_type() == PieceType::None
        }
        SfBinpackLoader::new_concat_multiple(
            &[
                r"H:\RazorBot\data\sf\test80-2024-01-jan.binpack",
                r"H:\RazorBot\data\sf\test80-2024-02-feb.binpack",
                r"H:\RazorBot\data\sf\test80-2024-03-mar.binpack",
                r"H:\RazorBot\data\sf\test80-2024-04-apr.binpack",
            ],
            1024,
            6,
            filter,
        )
    };

    trainer.run(&schedule, &settings, &data_loader);
}
