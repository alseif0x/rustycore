//! Read-only acquisition evidence. C++ Player::_SaveSpells omits dependent
//! spells; _LoadSkills -> LearnSkillRewardedSpells reconstructs them at login.
use super::*;
use mysql::prelude::Queryable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Expectation {
    DirectSpell {},
    Skill { id: u16, value: u16, max: u16 },
}

impl Default for Expectation {
    fn default() -> Self {
        Self::DirectSpell {}
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(super) struct SkillRoot {
    id: u16,
    value: u16,
    max: u16,
}

impl Expectation {
    pub(super) fn validate(self) -> Result<()> {
        if let Self::Skill { id, value, max } = self {
            if id == 0 || value == 0 || value > max {
                bail!("skill persistence requires a positive ID and value within maximum");
            }
        }
        Ok(())
    }

    fn matches(self, observed: &State) -> bool {
        match self {
            Self::DirectSpell {} => observed.saved_spell,
            Self::Skill { id, value, max } => {
                // A direct active spell row must not mask reconstruction from
                // the selected skill root during the next authentication.
                !observed.saved_spell && observed.skill_root == Some(SkillRoot { id, value, max })
            }
        }
    }

    pub(super) fn verify_login(self, observed: &State, known_at_login: bool) -> Result<()> {
        if !known_at_login || !self.matches(observed) {
            bail!("relogin lost acquired spell or its expected persistence root");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Evidence {
    pub(super) expected_spell: u32,
    pub(super) trainer_guid: Option<(u64, u64)>,
    pub(super) learned_on_instance: bool,
    pub(super) verified_at_login: bool,
    pub(super) money_before: u64,
    pub(super) observed_db_money_after_action: u64,
    pub(super) expected_saved_money: u64,
    pub(super) repeated_purchase_rejected: bool,
    /// Literal active, non-disabled target row observed after confirmed logout.
    pub(super) saved_spell: bool,
    pub(super) saved_money: u64,
    pub(super) persistence: Expectation,
    pub(super) observed_skill_root: Option<SkillRoot>,
    /// DB evidence matches the explicit contract; login knowledge is separate.
    pub(super) persistence_verified: bool,
}

pub(super) struct State {
    pub(super) money: u64,
    pub(super) saved_spell: bool,
    pub(super) skill_root: Option<SkillRoot>,
}

fn state(bot: &config::BotConfig, spell: u32, expectation: Expectation) -> Result<State> {
    let url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&url).map_err(|_| anyhow!("invalid QA DB options"))?;
    let mut conn = mysql::Conn::new(opts).map_err(|_| anyhow!("QA DB connection failed"))?;
    let money: Option<u64> = conn
        .exec_first(
            "SELECT money FROM characters WHERE guid=? AND account=?",
            (bot.character_guid, bot.account_id),
        )
        .map_err(|_| anyhow!("acquisition character read failed"))?;
    let money = money.context("acquisition character is missing")?;
    let learned: Option<u8> = conn
        .exec_first(
            "SELECT active FROM character_spell WHERE guid=? AND spell=? AND disabled=0",
            (bot.character_guid, spell),
        )
        .map_err(|_| anyhow!("acquisition spell read failed"))?;
    let skill_root = if let Expectation::Skill { id, .. } = expectation {
        let row: Option<(u16, u16)> = conn
            .exec_first(
                "SELECT value, max FROM character_skills WHERE guid=? AND skill=?",
                (bot.character_guid, id),
            )
            .map_err(|_| anyhow!("acquisition skill root read failed"))?;
        row.map(|(value, max)| SkillRoot { id, value, max })
    } else {
        None
    };
    Ok(State {
        money,
        saved_spell: learned == Some(1),
        skill_root,
    })
}

pub(super) async fn read_state(
    bot: &config::BotConfig,
    spell: u32,
    expectation: Expectation,
) -> Result<State> {
    let selected = bot.clone();
    tokio::task::spawn_blocking(move || state(&selected, spell, expectation)).await?
}

pub(crate) async fn verify_saved(bot: &config::BotConfig, receipt: &mut Evidence) -> Result<()> {
    let observed = read_state(bot, receipt.expected_spell, receipt.persistence).await?;
    if !receipt.persistence.matches(&observed) || observed.money != receipt.expected_saved_money {
        bail!("confirmed logout did not retain acquisition root and money");
    }
    receipt.saved_money = observed.money;
    receipt.saved_spell = observed.saved_spell;
    receipt.observed_skill_root = observed.skill_root;
    receipt.persistence_verified = true;
    Ok(())
}

#[cfg(test)]
mod tests;
