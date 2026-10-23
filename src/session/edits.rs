//! Voie de publication commune : valider le brouillon, normaliser, puis publier
//! contenu, sélection, révision et historique ensemble. Aucun effet externe ici.
use super::*;

impl Session {
    #[cfg(test)]
    pub fn remember(
        &mut self,
        before: Snapshot,
        selection: Selection,
        typing: bool,
    ) -> Result<Selection, EditError> {
        self.record(
            before,
            selection,
            if typing {
                EditGroup::Body
            } else {
                EditGroup::Discrete
            },
        )
    }

    /// Adaptateur transitoire des commandes qui préparent encore leurs champs
    /// dans App. Rétablir l'état publié avant toute validation du candidat.
    #[cfg(test)]
    pub fn record(
        &mut self,
        before: Snapshot,
        selection: Selection,
        group: EditGroup,
    ) -> Result<Selection, EditError> {
        if before.guard != self.target_guard() {
            return Err(EditError::StaleDraft);
        }
        let candidate = self.snapshot(selection);
        self.restore_fields(&before);
        self.publish(before, candidate, selection, group)
    }

    pub fn commit_projection(
        &mut self,
        guard: TargetGuard,
        before: Snapshot,
        view: Projection,
        selection: Selection,
        group: EditGroup,
    ) -> Result<Selection, EditError> {
        let mut candidate = before.clone();
        candidate.document.view = view;
        self.commit_snapshot(guard, before, candidate, selection, group)
    }

    /// Publication d’un candidat complet préparé sans modifier la session.
    pub fn commit_snapshot(
        &mut self,
        guard: TargetGuard,
        before: Snapshot,
        candidate: Snapshot,
        selection: Selection,
        group: EditGroup,
    ) -> Result<Selection, EditError> {
        if guard != self.target_guard() || before.guard != guard || candidate.guard != guard {
            return Err(EditError::StaleDraft);
        }
        self.publish(before, candidate, selection, group)
    }

    fn restore_fields(&mut self, snapshot: &Snapshot) {
        self.document = snapshot.document.clone();
        self.metadata = snapshot.metadata.clone();
        self.template = snapshot.template.clone();
        self.dependencies = snapshot.dependencies.clone();
    }

    fn publish(
        &mut self,
        before: Snapshot,
        mut candidate: Snapshot,
        mut selection: Selection,
        group: EditGroup,
    ) -> Result<Selection, EditError> {
        candidate.document.view.validate_edit()?;
        if !before.document.same_content(&candidate.document) {
            if candidate.document.view.is_empty()
                && !before.document.view.is_empty()
                && before.selection.range() == (0..before.document.view.len())
            {
                candidate
                    .document
                    .view
                    .blocks
                    .fill(crate::document::BlockKind::Paragraph);
            }
            for at in candidate.document.sync_source() {
                for cursor in [&mut selection.primary, &mut selection.secondary] {
                    if cursor.index > at {
                        cursor.index += 1;
                    }
                }
            }
            candidate.document.view.validate_edit()?;
        }
        for cursor in [&mut selection.primary, &mut selection.secondary] {
            cursor.index = cursor.index.min(candidate.document.view.len());
        }
        let body_changed = !before.document.same_content(&candidate.document);
        let changed = body_changed
            || before.metadata != candidate.metadata
            || before.template != candidate.template
            || before.dependencies != candidate.dependencies;
        if !body_changed {
            let typing_style = candidate.document.view.typing_style.clone();
            candidate.document = before.document.clone();
            candidate.document.view.typing_style = typing_style;
        }
        if !changed {
            self.restore_fields(&candidate);
            return Ok(selection);
        }
        if group == EditGroup::Discrete
            || !self.typing_group
            || self.last_group != group
            || self.last_edit.elapsed() > Duration::from_millis(650)
        {
            let mut before = before;
            // Certains raccourcis préparent un instantané avec leur saisie
            // littérale : recalculer la sélection après cette préparation.
            before.logical_selection = before
                .document
                .structure
                .selection_at(
                    before.selection.secondary.index,
                    before.selection.primary.index,
                )
                .ok()
                .map(|selection| SelectionTarget {
                    guard: self.target_guard(),
                    selection,
                });
            self.undo.push(before);
            if self.undo.len() > 100 {
                self.undo.remove(0);
            }
        }
        self.restore_fields(&candidate);
        self.redo.clear();
        self.typing_group = group != EditGroup::Discrete;
        self.last_group = group;
        self.last_edit = Instant::now();
        self.revision += 1;
        self.changed_at = Instant::now();
        Ok(selection)
    }

    pub fn edit(
        &mut self,
        mut selection: Selection,
        edit: impl FnOnce(&mut Projection, Range<usize>),
    ) -> Result<Selection, EditError> {
        for cursor in [&mut selection.primary, &mut selection.secondary] {
            cursor.index = cursor.index.min(self.document.view.len());
        }
        let guard = self.target_guard();
        let before = self.snapshot(selection);
        let mut view = self.document.view.clone();
        let range = selection.range();
        edit(&mut view, range.clone());
        if !range.is_empty() {
            view.typing_style = view.style_at(range.start);
        }
        self.commit_projection(guard, before, view, selection, EditGroup::Discrete)
    }

    pub fn replace_text(
        &mut self,
        target: SelectionTarget,
        text: &str,
    ) -> Result<Selection, EditError> {
        let (anchor, head) = self.resolve_target(target).map_err(EditError::Target)?;
        let typing_style = if anchor == head {
            self.document.view.typing_style.clone()
        } else {
            self.document.view.style_at(anchor.min(head))
        };
        let after = self.replace_ranges(
            Selection {
                primary: Caret {
                    index: head,
                    ..Default::default()
                },
                secondary: Caret {
                    index: anchor,
                    ..Default::default()
                },
            },
            std::slice::from_ref(&(anchor.min(head)..anchor.max(head))),
            text,
        )?;
        self.document.view.typing_style = typing_style;
        Ok(Selection {
            secondary: after.primary,
            ..after
        })
    }

    /// Les remplacements sont validés et publiés en une seule transaction.
    /// Une erreur dans un des résultats ne publie aucun des autres.
    pub fn replace_ranges(
        &mut self,
        selection: Selection,
        ranges: &[Range<usize>],
        replacement: &str,
    ) -> Result<Selection, EditError> {
        let Some(first) = ranges.first() else {
            return Ok(selection);
        };
        if ranges.windows(2).any(|pair| pair[0].end > pair[1].start) {
            return Err(EditError::InvalidSelection);
        }
        let guard = self.target_guard();
        let before = self.snapshot(selection);
        let mut input = TextInput {
            view: self.document.view.clone(),
        };
        let old_style = input.view.typing_style.clone();
        let mut inserted = 0;
        for range in ranges.iter().rev() {
            input.view.typing_style = if range.is_empty() {
                old_style.clone()
            } else {
                input.view.style_at(range.start)
            };
            input.delete_char_range(range.clone());
            inserted = input.insert_text(replacement, range.start);
        }
        input.view.typing_style = old_style;
        let after = Selection {
            primary: Caret {
                index: first.start + inserted,
                ..Default::default()
            },
            secondary: Caret {
                index: first.start,
                ..Default::default()
            },
        };
        self.commit_projection(guard, before, input.view, after, EditGroup::Discrete)
    }
}
