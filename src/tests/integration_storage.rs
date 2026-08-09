use memory_frame::{
    MemoryFrame,
    LayerId,
    SliceData,
    storage::{serialize_frame, deserialize_frame, InMemoryStore},
    config::defaults::default_policy,
};

#[test]
fn test_storage_roundtrip() {
    let mut frame = MemoryFrame::new(default_policy());
    frame.insert_slice(LayerId::Declarative, SliceData::Declarative("hello world".into()));

    let record = serialize_frame(&frame).unwrap();
    let restored = deserialize_frame(record).unwrap();

    assert_eq!(restored.slices.len(), frame.slices.len());
}
