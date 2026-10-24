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
