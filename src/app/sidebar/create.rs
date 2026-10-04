use std::fmt::Write as _;

use crate::{
    app::{Planner, TreeTarget},
    model::{Category, Group},
    text::Name,
};

use super::rename::RenameDraft;

#[derive(Clone, Copy)]
pub(super) enum CreateTarget {
    Group,
    Category(usize),
}

impl Planner {
    pub(super) fn create_tree_entry(&mut self, target: CreateTarget) {
        let language = self.language();
        let (target, name) = match target {
            CreateTarget::Group => {
                let name = unique_name(
                    language.text("Новая группа", "New group"),
                    |name| {
                        self.document
                            .groups
                            .iter()
                            .any(|group| group.name.get(language) == name)
                    },
                );
                let target = TreeTarget::Group(self.document.groups.len());
                self.document.groups.push(Group {
                    name: Name::custom(&name).expect("Nonempty default name"),
                    enabled: true,
                    expanded: true,
                    categories: Vec::new(),
                });
                (target, name)
            }
            CreateTarget::Category(index) => {
                let name = unique_name(
                    language.text("Новый календарь", "New calendar"),
                    |name| {
                        self.document
                            .categories()
                            .any(|category| category.name.get(language) == name)
                    },
                );
                let id = self.document.next_category_id();
                let Some(group) = self.document.groups.get_mut(index) else {
                    return;
                };
                group.categories.push(Category {
                    id,
                    name: Name::custom(&name).expect("Nonempty default name"),
                    enabled: true,
                    color: [39, 133, 245],
                });
                group.expanded = true;
                (TreeTarget::Category(id), name)
            }
        };
        self.rename_draft = Some(RenameDraft::selected(target, &name));
        self.mark_changed();
    }
}

fn unique_name(base: &str, exists: impl Fn(&str) -> bool) -> String {
    let mut name = base.to_owned();
    let mut suffix = 0_usize;
    while exists(&name) {
        suffix += 1;
        name.truncate(base.len());
        write!(name, " #{suffix}").expect("Writing to a string cannot fail");
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{model::Document, text::Language};

    #[test]
    fn default_names_are_unique_across_groups_and_reuse_available_suffixes() {
        let ctx = egui::Context::default();
        let mut planner = Planner::from_document(Document::default(), &ctx, None);
        for suffix in ["", " #1", " #2", " #3"] {
            planner.create_tree_entry(CreateTarget::Group);
            assert_eq!(
                planner
                    .document
                    .groups
                    .last()
                    .unwrap()
                    .name
                    .get(Language::Russian),
                format!("Новая группа{suffix}")
            );
            planner.create_tree_entry(CreateTarget::Category(0));
            assert_eq!(
                planner.document.groups[0]
                    .categories
                    .last()
                    .unwrap()
                    .name
                    .get(Language::Russian),
                format!("Новый календарь{suffix}")
            );
        }
        planner.document.groups[0]
            .categories
            .retain(|category| category.name.get(Language::Russian) != "Новый календарь #2");
        planner
            .document
            .groups
            .retain(|group| group.name.get(Language::Russian) != "Новая группа #2");
        planner.document.groups[1].expanded = false;
        planner.create_tree_entry(CreateTarget::Category(1));
        assert!(planner.document.groups[1].expanded);
        assert_eq!(
            planner.document.groups[1]
                .categories
                .last()
                .unwrap()
                .name
                .get(Language::Russian),
            "Новый календарь #2"
        );
        planner.create_tree_entry(CreateTarget::Group);
        assert_eq!(
            planner
                .document
                .groups
                .last()
                .unwrap()
                .name
                .get(Language::Russian),
            "Новая группа #2"
        );
        assert!(planner.persistence.is_dirty());
        assert!(planner.tree_draft.is_none());
        assert!(Document::parse(&serde_json::to_string(&planner.document).unwrap()).is_ok());
    }
}
