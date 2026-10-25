use photocraft_engine::actions_cmds::Action;
use photocraft_engine::atn_cmds;
use serde_json::json;

#[test]
fn fractional_vibrance_balance_is_rejected_not_rounded() {
    for key in ["temperature", "tint"] {
        for value in [-67.25, -0.5, 0.5, 67.25] {
            let action = Action::new("Balance", vec![("image.adjustments.vibrance".into(), json!({key: value}))]);
            assert!(atn_cmds::export("set", &[&action], &[], false).is_err(), "{key}: {value}");
        }
    }
}

#[test]
fn integral_vibrance_balance_round_trips() {
    for key in ["temperature", "tint"] {
        for value in [-100, -1, 0, 1, 100] {
            let params = json!({key: value});
            let action = Action::new("Balance", vec![("image.adjustments.vibrance".into(), params.clone())]);
            let (bytes, skipped) = atn_cmds::export("set", &[&action], &[], false).unwrap();
            assert!(skipped.is_empty());
            let imported = atn_cmds::import(&bytes).unwrap();
            assert_eq!(imported.actions[0].steps[0].1[key], params[key]);
        }
    }
}

fn adjustment_atn(key: &str, value: Option<f64>) -> Vec<u8> {
    use photocraft_psd::atn::{self, AtnAction, AtnEvent, AtnFile, AtnItem};
    use photocraft_psd::descriptor::{Class, Descriptor, Id, ReferenceItem, UnicodeString, Value as D};
    let mut balance = Descriptor::new("Vbrn").with("vibrance", D::Integer(12)).with("Strt", D::Integer(-5));
    if let Some(value) = value {
        balance = balance.with(key, D::Double(value));
    }
    let descriptor = Descriptor::new("null")
        .with("null", D::Reference(vec![ReferenceItem::Class(Class { name: UnicodeString::default(), class_id: Id::new("AdjL") })]))
        .with("Usng", D::Descriptor(Descriptor::new("AdjL").with("Type", D::Descriptor(balance))));
    atn::write(&AtnFile {
        name: UnicodeString::new_nul("Balance"),
        actions: vec![AtnAction {
            name: UnicodeString::new_nul("Imported"),
            items: vec![AtnItem { event: AtnEvent::name("make"), descriptor: Some(descriptor), ..Default::default() }],
            ..Default::default()
        }],
        ..Default::default()
    })
    .unwrap()
}

#[test]
fn imported_fractional_vibrance_balance_keeps_original_descriptor() {
    for key in ["temperature", "tint"] {
        for value in [-67.25, -0.5, 0.5, 67.25] {
            let bytes = adjustment_atn(key, Some(value));
            let imported = atn_cmds::import(&bytes).unwrap();
            let (saved, skipped) = atn_cmds::export(&imported.set, &[&imported.actions[0]], &[], false).unwrap();
            assert!(skipped.is_empty());
            let original = photocraft_psd::atn::parse(&bytes).unwrap();
            let restored = photocraft_psd::atn::parse(&saved).unwrap();
            assert_eq!(restored.actions[0].items[0].descriptor, original.actions[0].items[0].descriptor, "{key}: {value}");
        }
    }
}

#[test]
fn imported_integral_and_missing_vibrance_balance_stays_typed() {
    for key in ["temperature", "tint"] {
        for value in [None, Some(-100.0), Some(0.0), Some(100.0)] {
            let imported = atn_cmds::import(&adjustment_atn(key, value)).unwrap();
            assert!(imported.unsupported.is_empty());
            assert_eq!(imported.actions[0].steps[0].0, "layer.newAdjustmentLayer.vibrance");
            let (_, skipped) = atn_cmds::export(&imported.set, &[&imported.actions[0]], &[], false).unwrap();
            assert!(skipped.is_empty());
        }
    }
}

#[test]
fn existing_exact_long_filters_preserve_fractional_imports_and_typed_controls() {
    use photocraft_psd::atn::{self, AtnAction, AtnEvent, AtnFile, AtnItem};
    use photocraft_psd::descriptor::{Descriptor, UnicodeString, Value as D};
    for (event, key, command) in [("Rple", "Amnt", "filter.distort.ripple"), ("Crst", "ClSz", "filter.pixelate.crystallize")] {
        for value in [None, Some(0.0), Some(5.0), Some(-0.5), Some(4.25), Some(f64::from(i32::MAX) + 1.0)] {
            let mut descriptor = Descriptor::new("null");
            if let Some(value) = value {
                descriptor = descriptor.with(key, D::Double(value));
            }
            let bytes = atn::write(&AtnFile {
                name: UnicodeString::new_nul("Controls"),
                actions: vec![AtnAction {
                    name: UnicodeString::new_nul("Imported"),
                    items: vec![AtnItem {
                        event: AtnEvent::Code(event.as_bytes().try_into().unwrap()),
                        descriptor: Some(descriptor.clone()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            })
            .unwrap();
            let imported = atn_cmds::import(&bytes).unwrap();
            let unsupported = value.is_some_and(|n| f64::from(n as i32) != n);
            assert_eq!(imported.unsupported.len(), usize::from(unsupported), "{event}: {value:?}");
            if !unsupported {
                assert_eq!(imported.actions[0].steps[0].0, command);
            }
            let (saved, skipped) = atn_cmds::export(&imported.set, &[&imported.actions[0]], &[], false).unwrap();
            assert!(skipped.is_empty());
            let restored = atn::parse(&saved).unwrap();
            if unsupported {
                assert_eq!(restored.actions[0].items[0].descriptor, Some(descriptor), "{event}: {value:?}");
            }
        }
    }
}
