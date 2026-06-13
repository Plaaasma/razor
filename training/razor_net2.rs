/*
Razor net 2: same (768 -> 512)x2 -> 1 SCReLU arch as net1, but trained on gen2
(100M positions labeled by the v0.5.0 NNUE engine, vs net1's PSQT-labeled gen1).
Same arch on purpose — isolates the data-quality improvement. See STATE.md Phase 3.
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

const HIDDEN_SIZE: usize = 512;
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
        net_id: "razor2".to_string(),
        eval_scale: SCALE as f32,
        steps: TrainingSteps {
            batch_size: 16_384,
            batches_per_superbatch: 6104,
            start_superbatch: 1,
            end_superbatch: 40,
        },
        wdl_scheduler: wdl::LinearWDL { start: 0.5, end: 0.8 },
        lr_scheduler: lr::StepLR { start: 0.001, gamma: 0.3, step: 15 },
        save_rate: 10,
    };

    let settings =
        LocalSettings { threads: 4, test_set: None, output_directory: "checkpoints_razor2", batch_queue_size: 64 };

    let data_loader = loader::DirectSequentialDataLoader::new(&[r"H:\RazorBot\data\gen2.bin"]);

    trainer.run(&schedule, &settings, &data_loader);
}
