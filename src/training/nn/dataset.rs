use burn::data::dataset::Dataset;

use crate::training::nn::topology::KrkItem;

#[derive(Clone, Debug)]
pub struct KrkDataset {
    pub items: Vec<KrkItem>,
}

impl KrkDataset {
    pub fn new(items: Vec<KrkItem>) -> Self {
        Self { items }
    }
}

impl Dataset<KrkItem> for KrkDataset {
    fn get(&self, index: usize) -> Option<KrkItem> {
        self.items.get(index).cloned()
    }

    fn len(&self) -> usize {
        self.items.len()
    }
}