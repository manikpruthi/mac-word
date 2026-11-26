//! The single-page About dialog and the separate language-data licences.

use craft::ui::contract::{Show as _, ShowContainer as _};
use craft::ui::dialog::{Dialog, DialogId, DialogResult, DialogStack, StackDialog, dialog_id};
use craft::ui::display::Text;
use craft::ui::patterns::form_dialogs::AboutWords;
use craft::ui::runtime::env::{Env, Label};
use craft::ui::sections::{DialogRole, FooterButton};
use egui::{Context, Ui};
use serde_json::{Value, json};

use super::host::{self, DialogHooks, Settle};
use crate::WordApp;

#[derive(Clone, Copy)]
pub(super) enum Information {
    About,
    LanguageData,
}

impl Information {
    fn dialog_key(self) -> &'static str {
        match self {
            Self::About => "help.about",
            Self::LanguageData => "languageData",
        }
    }
}

impl StackDialog for Information {
    type Result = ();

    fn id(&self) -> DialogId {
        dialog_id(self.dialog_key())
    }

    fn key(&self) -> Option<&str> {
        Some(self.dialog_key())
    }

    fn json_fields(&self) -> Option<Value> {
        match self {
            Self::About => Some(json!({ "tab": "about" })),
            Self::LanguageData => None,
        }
    }

    fn confirm(&mut self, _: &Context) -> Option<()> {
        Some(())
    }

    fn show(&mut self, ui: &mut Ui, _: &DialogStack) -> Option<DialogResult<()>> {
        let close = [FooterButton::new(Label::from(tl!("Close")), DialogRole::Accept)];
        let dialog = match self {
            Self::About => {
                let info = crate::credits::about_info();
                let words = Env::of(ui).words::<AboutWords>();
                let title = Label::from(words.title.fill(&[("app", info.app_name.as_str())]));
                let version = Label::from(words.version.fill(&[("version", info.version.as_str())]));
                Dialog::new(title).buttons(close).close_on_enter(true).show(ui, |ui, _| {
                    Text::new(version.clone()).show(ui);
                    Text::new(info.blurb.clone()).show(ui);
                    Text::new(info.built_with.clone()).show(ui);
                })
            }
            Self::LanguageData => Dialog::new(tl!("Language data")).buttons(close).close_on_enter(true).collection_body().show(ui, |ui, _| {
                egui::ScrollArea::vertical().show(ui, crate::language_packs::language_data);
            }),
        };
        dialog.output.closed.map(|_| DialogResult::Ok(()))
    }
}

pub(super) fn open(app: &mut WordApp, dialog: Information) -> Option<DialogId> {
    let id = app.dialog_host.open_unless_open(dialog).ok()?;
    host::register(app, id, &[DialogHooks { prepare: None, settle }]);
    Some(id)
}

fn settle(app: &mut WordApp, _: &Context, id: DialogId) -> Settle {
    if app.dialog_host.take_by_id::<Information>(id).is_some() { Settle::Done } else { Settle::Keep }
}
