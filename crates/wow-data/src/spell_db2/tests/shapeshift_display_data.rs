//! Opt-in diagnostic for the installed DB2 links used by C++ shapeshift model selection.
//!
//! Production readers establish row semantics. The existing WDC4 reader supplies the table
//! hash. This diagnostic follows `DB2Stores.cpp:1025-1102`; it does not apply SQL overlays or
//! evaluate a character's live customization requirements.

use super::*;
use std::collections::{BTreeMap, HashMap};

#[test]
#[ignore = "requires RUSTYCORE_RESURRECTION_DATA_DIR pointing at installed Data"]
fn shapeshift_display_data() {
    let data_dir = std::env::var("RUSTYCORE_RESURRECTION_DATA_DIR")
        .expect("set RUSTYCORE_RESURRECTION_DATA_DIR to the installed Data directory");
    let locale = "enUS";
    let data_path = Path::new(&data_dir).join("dbc").join(locale);
    let display_path = data_path.join("ChrCustomizationDisplayInfo.db2");
    assert!(
        display_path.is_file(),
        "ChrCustomizationDisplayInfo.db2 not found at {}",
        display_path.display()
    );

    use crate::character_progression::{
        ChrCustomizationChoiceStore, ChrCustomizationDisplayInfoStore,
        ChrCustomizationElementStore, ChrCustomizationOptionStore,
        ChrCustomizationReqChoiceStore, ChrRaceXChrModelStore,
    };

    let forms = SpellShapeshiftFormStore::load(&data_dir, locale)
        .expect("load SpellShapeshiftForm.db2");
    let choices = ChrCustomizationChoiceStore::load(&data_dir, locale)
        .expect("load ChrCustomizationChoice.db2");
    let displays = ChrCustomizationDisplayInfoStore::load(&data_dir, locale)
        .expect("load ChrCustomizationDisplayInfo.db2");
    let elements = ChrCustomizationElementStore::load(&data_dir, locale)
        .expect("load ChrCustomizationElement.db2");
    let options = ChrCustomizationOptionStore::load(&data_dir, locale)
        .expect("load ChrCustomizationOption.db2");
    let req_choices = ChrCustomizationReqChoiceStore::load(&data_dir, locale)
        .expect("load ChrCustomizationReqChoice.db2");
    let race_models = ChrRaceXChrModelStore::load(&data_dir, locale)
        .expect("load ChrRaceXChrModel.db2");
    let display_reader = crate::wdc4::Wdc4Reader::open(&display_path)
        .expect("open ChrCustomizationDisplayInfo.db2 with WDC4 reader");

    let mut nonzero_display_rows: Vec<_> = displays
        .entries()
        .filter(|row| row.shapeshift_form_id != 0)
        .collect();
    nonzero_display_rows.sort_unstable_by_key(|row| row.id);

    // This matches DB2Stores.cpp: displayInfoByCustomizationChoice is populated from
    // Element rows only when both the referenced DisplayInfo and Choice exist; the Choice's
    // Option then supplies the ChrModelID used by the race/gender join.
    let mut options_by_model = HashMap::<u32, Vec<(u32, i32, u32, u32, u32)>>::new();
    let mut linked_elements = 0usize;
    let mut linked_options = 0usize;
    let mut nonzero_form_links = 0usize;
    for element in elements.entries() {
        let Ok(choice_id) = u32::try_from(element.chr_customization_choice_id) else {
            continue;
        };
        let Ok(display_info_id) = u32::try_from(element.chr_customization_display_info_id) else {
            continue;
        };
        let (Some(choice), Some(display)) = (choices.get(choice_id), displays.get(display_info_id))
        else {
            continue;
        };
        linked_elements += 1;
        let Some(option) = options.get(choice.chr_customization_option_id) else {
            continue;
        };
        linked_options += 1;
        if display.shapeshift_form_id == 0 {
            continue;
        }
        nonzero_form_links += 1;
        options_by_model
            .entry(option.chr_model_id)
            .or_default()
            .push((
                option.id,
                display.shapeshift_form_id,
                choice_id,
                display_info_id,
                element.id,
            ));
    }

    let mut druid_links = Vec::new();
    for race_model in race_models
        .entries()
        .filter(|row| row.chr_races_id == 4 && matches!(row.sex, 0 | 1))
    {
        let Some(model_links) = options_by_model.get(&(race_model.chr_model_id as u32)) else {
            continue;
        };
        for &(option_id, form_id, choice_id, display_info_id, element_id) in model_links {
            let choice = choices.get(choice_id).expect("linked choice remains present");
            let display = displays
                .get(display_info_id)
                .expect("linked display remains present");
            let mut requirement_ids: Vec<_> = req_choices
                .entries()
                .filter(|row| row.chr_customization_choice_id == choice_id as i32)
                .map(|row| row.chr_customization_req_id)
                .collect();
            requirement_ids.sort_unstable();
            druid_links.push((
                race_model.sex,
                form_id,
                race_model.chr_model_id,
                option_id,
                choice_id,
                element_id,
                display.id,
                display.display_id,
                choice.chr_customization_req_id,
                choice.chr_customization_vis_req_id,
                requirement_ids,
            ));
        }
    }
    druid_links.sort_unstable_by_key(|row| {
        (row.0, row.1, row.2, row.3, row.4, row.5, row.6)
    });

    let mut form_display_rows: Vec<_> = forms
        .entries_like_cpp()
        .filter(|row| row.creature_display_id.iter().any(|display| *display != 0))
        .map(|row| (row.id, row.creature_display_id))
        .collect();
    form_display_rows.sort_unstable_by_key(|row| row.0);
    let mut druid_counts = BTreeMap::from([(0, 0usize), (1, 0usize)]);
    for row in &druid_links {
        *druid_counts.get_mut(&row.0).expect("native gender key") += 1;
    }
    let druid_choices_with_req = druid_links.iter().filter(|row| row.8 != 0).count();
    let druid_choices_with_visibility_req = druid_links.iter().filter(|row| row.9 != 0).count();
    let druid_choices_with_req_choice_rows = druid_links
        .iter()
        .filter(|row| !row.10.is_empty())
        .count();

    println!("# locale\t{locale}");
    println!(
        "# tableHash\tChrCustomizationDisplayInfo.db2\t0x{:08X}",
        display_reader.table_hash(),
    );
    println!(
        "# parsedRows\tSpellShapeshiftForm={}\tChrCustomizationElement={}\tChrCustomizationDisplayInfo={}\tChrCustomizationChoice={}\tChrCustomizationOption={}\tChrCustomizationReqChoice={}\tChrRaceXChrModel={}",
        forms.len(),
        elements.len(),
        displays.len(),
        choices.len(),
        options.len(),
        req_choices.len(),
        race_models.len(),
    );
    println!(
        "# linkCounts\tElementToDisplayAndChoice={linked_elements}\tElementToOption={linked_options}\tNonzeroShapeshiftForm={nonzero_form_links}\tDruidRace4Gender0={}\tDruidRace4Gender1={}",
        druid_counts[&0],
        druid_counts[&1],
    );
    println!(
        "# requirements\tDruidLinkedChoicesWithChoiceReq={druid_choices_with_req}\tDruidLinkedChoicesWithVisibilityReq={druid_choices_with_visibility_req}\tDruidLinkedChoicesWithReqChoiceRows={druid_choices_with_req_choice_rows}"
    );

    println!("ChrCustomizationDisplayInfo\tRecordID\tShapeshiftFormID\tDisplayID");
    for row in &nonzero_display_rows {
        println!(
            "ChrCustomizationDisplayInfo\t{}\t{}\t{}",
            row.id, row.shapeshift_form_id, row.display_id,
        );
    }
    println!("SpellShapeshiftForm\tFormID\tCreatureDisplayIDs");
    for (form_id, display_ids) in form_display_rows {
        println!(
            "SpellShapeshiftForm\t{form_id}\t{}",
            display_ids
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(","),
        );
    }
    println!(
        "DruidCustomization\tGender\tFormID\tChrModelID\tOptionID\tChoiceID\tElementID\tDisplayInfoID\tDisplayID\tChoiceReqID\tChoiceVisReqID\tChrCustomizationReqChoiceReqIDs"
    );
    for row in &druid_links {
        println!(
            "DruidCustomization\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.0,
            row.1,
            row.2,
            row.3,
            row.4,
            row.5,
            row.6,
            row.7,
            row.8,
            row.9,
            row.10
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(","),
        );
    }

    assert!(!forms.is_empty(), "SpellShapeshiftForm.db2 loaded no rows");
    assert!(
        nonzero_display_rows.is_empty(),
        "installed dataset has shapeshift customization display rows; do not enable the fallback selector without resolving the DB2 link chain"
    );
}
