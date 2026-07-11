use burn::{
    data::dataloader::batcher::Batcher,
    tensor::{
        backend::Backend,
        Tensor,
        TensorData,
    },
};

use crate::training::nn::topology::KrkItem;

#[derive(Clone, Debug)]
pub struct KrkBatch<B: Backend> {
    pub positions: Tensor<B, 2>,
    pub targets: Tensor<B, 2>,
}

#[derive(Clone, Debug, Default)]
pub struct KrkBatcher;

impl<B: Backend> Batcher<B, KrkItem, KrkBatch<B>> for KrkBatcher {
    fn batch(
        &self,
        items: Vec<KrkItem>,
        device: &B::Device,
    ) -> KrkBatch<B> {
        let batch_size = items.len();

        let positions = items
            .iter()
            .flat_map(|item| item.features)
            .collect::<Vec<f32>>();

        let targets = items
            .iter()
            .flat_map(|item| item.target)
            .collect::<Vec<f32>>();

        let positions = Tensor::<B, 2>::from_data(
            TensorData::new(positions, [batch_size, 192]),
            device,
        );

        let targets = Tensor::<B, 2>::from_data(
            TensorData::new(targets, [batch_size, 2]),
            device,
        );

        KrkBatch {
            positions,
            targets,
        }
    }
}
