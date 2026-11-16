use crate::player::skills::{PLAYER_MAX_SKILLS, PlayerSkills, SkillUpdateState};

fn mask_with(bits: &[usize]) -> [u32; 66] {
    let mut words = [0; 66];
    for &bit in bits {
        words[bit / 32] |= 1 << (bit % 32);
    }
    words
}

fn has_bit(words: &[u32; 66], bit: usize) -> bool {
    words[bit / 32] & (1 << (bit % 32)) != 0
}

#[test]
fn initialize_marks_all_written_fields_and_the_final_valid_mask_bit() {
    let mut requests = 0;
    let mut skills = PlayerSkills::initialize(0..PLAYER_MAX_SKILLS as u32, || requests += 1);

    // Line differs for slots 1..299; StartingRank differs for all 300 slots.
    assert_eq!(requests, 599);
    assert_eq!(skills.fields()[299].line(), 299);
    assert_eq!(skills.fields()[299].starting_rank(), 1);
    let mut expected = [0; 66];
    for bit in [0].into_iter().chain(1..=300).chain(901..=1200) {
        expected[bit / 32] |= 1 << (bit % 32);
    }
    assert_eq!(skills.field_changes(), &expected);

    skills.set_permanent_bonus(299, 7, || requests += 1);
    assert_eq!(requests, 600);
    expected[2100 / 32] |= 1 << (2100 % 32);
    assert_eq!(skills.field_changes(), &expected);
    assert_eq!(skills.field_changes()[65], 1 << 20); // SkillInfo bit 2100.
    assert!(has_bit(skills.field_changes(), 0));
    assert!(has_bit(skills.field_changes(), 300));
    assert!(has_bit(skills.field_changes(), 1200));
    assert!(has_bit(skills.field_changes(), 2100));
}

#[test]
fn equal_field_and_profession_writes_mark_history_without_requesting_object_admission() {
    let mut skills = PlayerSkills::initialize([42], || {});
    skills.clear_changes();
    let mut requests = 0;

    skills.set_line(0, 42, || requests += 1);
    skills.set_step(0, 0, || requests += 1);
    skills.set_rank(0, 0, || requests += 1);
    skills.set_starting_rank(0, 1, || requests += 1);
    skills.set_maximum(0, 0, || requests += 1);
    skills.set_temporary_bonus(0, 0, || requests += 1);
    skills.set_permanent_bonus(0, 0, || requests += 1);
    skills.set_profession_line(0, 0, || requests += 1);

    assert_eq!(requests, 0);
    assert_eq!(
        skills.field_changes(),
        &mask_with(&[0, 1, 301, 601, 901, 1201, 1501, 1801])
    );
    assert_eq!(skills.profession_changes(), 1);

    skills.set_profession_line(0, 42, || requests += 1);
    assert_eq!(requests, 1);
    assert_eq!(skills.profession_lines(), &[42, 0]);
}

#[test]
fn snapshot_freezes_values_and_masks_while_ack_and_clear_remain_independent() {
    let mut skills = PlayerSkills::initialize([10], || {});
    skills.set_step(0, 2, || {});
    skills.set_rank(0, 7, || {});
    skills.set_maximum(0, 75, || {});
    skills.set_profession_line(0, 10, || {});
    skills.status.get_mut(&10).unwrap().state = SkillUpdateState::Changed;

    let mask_before_ack = *skills.field_changes();
    let batch = skills
        .capture_save(|history, _| Ok::<_, ()>(history.to_vec()))
        .unwrap();
    skills.acknowledge_save(batch);
    assert_eq!(skills.status(10), Some((0, SkillUpdateState::Unchanged)));
    assert_eq!(skills.field_changes(), &mask_before_ack);

    let mut snapshot = skills.values_snapshot();
    skills.set_rank(0, 8, || {});
    skills.set_profession_line(1, 20, || {});
    skills.status.get_mut(&10).unwrap().state = SkillUpdateState::New;
    skills.clear_changes();

    assert_eq!(snapshot.fields()[0].rank(), 7);
    assert_eq!(snapshot.profession_lines(), &[10, 0]);
    assert_eq!(snapshot.field_changes(), &mask_before_ack);
    assert_eq!(snapshot.profession_changes(), 1);
    assert_eq!(skills.field_changes(), &[0; 66]);
    assert_eq!(skills.profession_changes(), 0);
    assert_eq!(skills.status(10), Some((0, SkillUpdateState::New)));

    snapshot.clear_changes();
    assert_eq!(snapshot.fields()[0].rank(), 7);
    assert_eq!(snapshot.profession_lines(), &[10, 0]);
    assert_eq!(snapshot.field_changes(), &[0; 66]);
    assert_eq!(snapshot.profession_changes(), 0);
}
