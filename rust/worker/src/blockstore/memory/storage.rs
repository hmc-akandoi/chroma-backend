use crate::{
    blockstore::key::{CompositeKey, KeyWrapper},
    segment::DataRecord,
};
use arrow::array::Int32Array;
use parking_lot::RwLock;
use roaring::RoaringBitmap;
use std::{collections::HashMap, fmt::Write, sync::Arc};

pub(crate) trait Writeable {
    fn write_to_storage(prefix: &str, key: KeyWrapper, value: Self, storage: &StorageBuilder);
}

pub(crate) trait Readable<'referred_data>: Sized {
    fn read_from_storage(
        prefix: &str,
        key: KeyWrapper,
        storage: &'referred_data Storage,
    ) -> Option<Self>;
}

impl Writeable for &str {
    fn write_to_storage(prefix: &str, key: KeyWrapper, value: Self, storage: &StorageBuilder) {
        storage
            .string_value_storage
            .write()
            .as_mut()
            .unwrap()
            .insert(
                CompositeKey {
                    prefix: prefix.to_string(),
                    key,
                },
                value.to_string(),
            );
    }
}

impl<'referred_data> Readable<'referred_data> for &'referred_data str {
    fn read_from_storage(
        prefix: &str,
        key: KeyWrapper,
        storage: &'referred_data Storage,
    ) -> Option<Self> {
        storage
            .string_value_storage
            .get(&CompositeKey {
                prefix: prefix.to_string(),
                key,
            })
            .map(|s| s.as_str())
    }
}

// TODO: remove this and make this all use a unified storage so we don't have two impls
impl Writeable for &Int32Array {
    fn write_to_storage(prefix: &str, key: KeyWrapper, value: Self, storage: &StorageBuilder) {
        todo!()
    }
}

impl Readable<'_> for Int32Array {
    fn read_from_storage(prefix: &str, key: KeyWrapper, storage: &Storage) -> Option<Self> {
        todo!()
    }
}

impl Writeable for &RoaringBitmap {
    fn write_to_storage(prefix: &str, key: KeyWrapper, value: Self, storage: &StorageBuilder) {
        todo!()
    }
}

impl Writeable for u32 {
    fn write_to_storage(prefix: &str, key: KeyWrapper, value: Self, storage: &StorageBuilder) {
        todo!()
    }
}

impl Readable<'_> for u32 {
    fn read_from_storage(prefix: &str, key: KeyWrapper, storage: &Storage) -> Option<Self> {
        todo!()
    }
}

impl Readable<'_> for RoaringBitmap {
    fn read_from_storage(prefix: &str, key: KeyWrapper, storage: &Storage) -> Option<Self> {
        todo!()
    }
}

impl Writeable for &DataRecord<'_> {
    fn write_to_storage(prefix: &str, key: KeyWrapper, value: Self, storage: &StorageBuilder) {
        storage
            .data_record_id_storage
            .write()
            .as_mut()
            .unwrap()
            .insert(
                CompositeKey {
                    prefix: prefix.to_string(),
                    key: key.clone(),
                },
                value.id.to_string(),
            );
        storage
            .data_record_embedding_storage
            .write()
            .as_mut()
            .unwrap()
            .insert(
                CompositeKey {
                    prefix: prefix.to_string(),
                    key,
                },
                value.embedding.to_vec(),
            );
    }
}

impl<'referred_data> Readable<'referred_data> for DataRecord<'referred_data> {
    fn read_from_storage(
        prefix: &str,
        key: KeyWrapper,
        storage: &'referred_data Storage,
    ) -> Option<Self> {
        let id = storage.data_record_id_storage.get(&CompositeKey {
            prefix: prefix.to_string(),
            key: key.clone(),
        });
        let embedding = storage.data_record_embedding_storage.get(&CompositeKey {
            prefix: prefix.to_string(),
            key,
        });
        // TODO: don't unwrap
        Some(DataRecord {
            id: &id.unwrap(),
            embedding: &embedding.unwrap(),
            metadata: None,
            document: None,
        })
    }
}

// Int32ArrayValue(Int32Array),
// PositionalPostingListValue(PositionalPostingList),
// StringValue(String),
// IntValue(i32),
// UintValue(u32),
// RoaringBitmapValue(RoaringBitmap),

// pub(crate) struct DataRecord<'a> {
//     pub(crate) id: &'a str,
//     pub(crate) embedding: &'a [f32],
//     pub(crate) metadata: &'a Option<Metadata>,
//     pub(crate) document: &'a Option<String>,
//     // Optional staged serialized version of the metadata
//     pub(crate) serialized_metadata: Option<Vec<u8>>,
// }

#[derive(Clone)]
pub(crate) struct StorageBuilder {
    // String Value
    string_value_storage: Arc<RwLock<Option<HashMap<CompositeKey, String>>>>,
    // Data Record Fields
    data_record_id_storage: Arc<RwLock<Option<HashMap<CompositeKey, String>>>>,
    data_record_embedding_storage: Arc<RwLock<Option<HashMap<CompositeKey, Vec<f32>>>>>,
    pub(super) id: uuid::Uuid,
}

#[derive(Clone)]
pub(crate) struct Storage {
    // String Value
    string_value_storage: Arc<HashMap<CompositeKey, String>>,
    // Data Record Fields
    data_record_id_storage: Arc<HashMap<CompositeKey, String>>,
    data_record_embedding_storage: Arc<HashMap<CompositeKey, Vec<f32>>>,
    pub(super) id: uuid::Uuid,
}

#[derive(Clone)]
pub(crate) struct StorageManager {
    read_cache: Arc<RwLock<HashMap<uuid::Uuid, Storage>>>,
    write_cache: Arc<RwLock<HashMap<uuid::Uuid, StorageBuilder>>>,
}

impl StorageManager {
    pub(super) fn new() -> Self {
        Self {
            read_cache: Arc::new(RwLock::new(HashMap::new())),
            write_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub(super) fn get(&self, id: uuid::Uuid) -> Option<Storage> {
        let cache_guard = self.read_cache.read();
        let storage = cache_guard.get(&id)?.clone();
        Some(storage)
    }

    pub(super) fn create(&self) -> StorageBuilder {
        let id = uuid::Uuid::new_v4();
        let builder = StorageBuilder {
            string_value_storage: Arc::new(RwLock::new(Some(HashMap::new()))),
            data_record_id_storage: Arc::new(RwLock::new(Some(HashMap::new()))),
            data_record_embedding_storage: Arc::new(RwLock::new(Some(HashMap::new()))),
            id,
        };
        let mut cache_guard = self.write_cache.write();
        cache_guard.insert(id, builder.clone());
        builder
    }

    pub(super) fn commit(&self, id: uuid::Uuid) -> Storage {
        let mut write_cache_guard = self.write_cache.write();
        let builder = write_cache_guard.remove(&id).unwrap();
        let storage = Storage {
            string_value_storage: builder.string_value_storage.write().take().unwrap().into(),
            data_record_id_storage: builder
                .data_record_id_storage
                .write()
                .take()
                .unwrap()
                .into(),
            data_record_embedding_storage: builder
                .data_record_embedding_storage
                .write()
                .take()
                .unwrap()
                .into(),
            id,
        };
        let mut read_cache_guard = self.read_cache.write();
        read_cache_guard.insert(id, storage.clone());
        storage
    }
}
