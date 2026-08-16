use memory_frame::{
    MemoryFrame,
    LayerId,
    SliceData,
    frame::NavTarget,
    config::defaults::default_policy,
};

#[test]
fn test_navigation() {
    let mut frame = MemoryFrame::new(default_policy());

    frame.insert_slice(LayerId::Visual, SliceData::Visual(vec![]));
    frame.insert_slice(LayerId::Semantic, SliceData::Semantic(serde_json::json!({})));

    let next = frame.navigate(Some(LayerId::Visual), NavTarget::NextSlice).unwrap();
    assert_eq!(next, LayerId::Semantic);

    let prev = frame.navigate(Some(LayerId::Semantic), NavTarget::PrevSlice).unwrap();
    assert_eq!(prev, LayerId::Visual);
}






