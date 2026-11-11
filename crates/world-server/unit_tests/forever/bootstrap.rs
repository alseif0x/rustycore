use super::navigation_arguments;
use std::ffi::{OsStr, OsString};

#[test]
fn navigation_requires_all_three_values_without_advancing_on_failure() {
    for flag in [
        "--ack-available-navigation56",
        "--ack-available-navigation57",
    ] {
        let args: Vec<OsString> = [flag, "package", "plan"].map(OsString::from).into();
        for length in 1..=args.len() {
            let mut offset = 0;
            assert!(navigation_arguments(&args[..length], &mut offset).is_err());
            assert_eq!(offset, 0);
        }
    }
}

#[test]
fn navigation_consumes_flag_and_values_and_preserves_capture_arguments() {
    for flag in [
        "--ack-available-navigation56",
        "--ack-available-navigation57",
    ] {
        for capture in [false, true] {
            let mut args: Vec<OsString> = ["previous", flag, "package", "plan", "48"]
                .map(OsString::from)
                .into();
            if capture {
                args.extend(
                    ["--ack-private-character-create-capture", "private-file"].map(OsString::from),
                );
            }
            let mut offset = 1;
            assert_eq!(
                navigation_arguments(&args, &mut offset).unwrap(),
                Some([OsStr::new("package"), OsStr::new("plan"), OsStr::new("48")])
            );
            assert_eq!(offset, 5);
            assert_eq!(args.len() - offset, if capture { 2 } else { 0 });
        }
    }
}

#[test]
fn absent_navigation_leaves_other_options_untouched() {
    let args = [OsString::from("--ack-private-name-availability-capture")];
    let mut offset = 0;
    assert!(navigation_arguments(&args, &mut offset).unwrap().is_none());
    assert_eq!(offset, 0);
    offset = args.len();
    assert!(navigation_arguments(&args, &mut offset).unwrap().is_none());
    offset += 1;
    assert!(navigation_arguments(&args, &mut offset).is_err());
}
