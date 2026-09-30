use super::*;

#[test]
fn void_storage_login_context_preserves_cpp_field_five_bug() {
    let selected_context_column = ItemContext::Timewalking as u8;

    assert_eq!(
        void_storage_login_context_like_cpp(29, selected_context_column),
        29
    );
    assert_ne!(29, selected_context_column);
}
