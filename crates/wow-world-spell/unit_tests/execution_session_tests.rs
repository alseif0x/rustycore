use super::server_normal_pre_go_failure_publication_like_cpp;
use crate::ServerNormalPreGoFailureLikeCpp;
use wow_constants::SpellCastResult;

#[test]
fn normal_pre_go_unavailable_is_propagated_but_cast_rejections_are_not() {
    let unavailable = server_normal_pre_go_failure_publication_like_cpp(
        ServerNormalPreGoFailureLikeCpp::Unavailable("SpellHistory unavailable"),
    );
    assert_eq!(
        unavailable,
        (
            SpellCastResult::SpellUnavailable as i32,
            0,
            0,
            Some("SpellHistory unavailable"),
        )
    );

    for (failure, expected) in [
        (
            ServerNormalPreGoFailureLikeCpp::MissingReagent(8_467),
            (SpellCastResult::Reagents as i32, 8_467, 0),
        ),
        (
            ServerNormalPreGoFailureLikeCpp::MissingCurrency(395),
            (SpellCastResult::Reagents as i32, -1, 395),
        ),
        (
            ServerNormalPreGoFailureLikeCpp::Power,
            (SpellCastResult::DontReport as i32, 0, 0),
        ),
    ] {
        let (reason, arg1, arg2, propagated) =
            server_normal_pre_go_failure_publication_like_cpp(failure);
        assert_eq!((reason, arg1, arg2), expected);
        assert_eq!(propagated, None);
    }
}
