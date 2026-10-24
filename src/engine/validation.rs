//! Validation documentaire sans publication. Un champ requis à l'export absent
//! est un avertissement de préparation, pas une erreur de brouillon.
use super::*;

impl Engine {
    pub fn validate(&self, p: GuardParams) -> Result<ValidationResult, CommandError> {
        self.check_rule(AvailabilityRule::Read)?;
        self.check_guard(p.guard)?;
        let mut diagnostics = Vec::new();
        let mut stored = self.stored_document();
        // Valider ce qui serait sauvegardé, pas les ressources de l'annulation.
        match crate::rendering::retained_assets(
            &stored.source,
            &stored.metadata,
            &stored.template,
            &stored.assets,
            &stored.dependencies,
        ) {
            Ok(retained) => stored.assets = retained,
            Err(detail) => diagnostics.push(Diagnostic::error("asset_inventory", detail)),
        }
        if let Err(detail) = crate::storage::validate(&stored) {
            diagnostics.push(Diagnostic::error("document_storage", detail));
        } else {
            let mut world = crate::rendering::TypstWorld::default();
            world.set_assets(&stored.assets);
            world.set_dependencies(&stored.dependencies);
            if let Err(detail) =
                world.validate_model(&stored.source, &stored.metadata, &stored.template)
            {
                diagnostics.push(Diagnostic::error("model_validation", detail));
            }
        }
        let options = self.metadata().options();
        for option in [
            DocumentOption::Paper(options.paper),
            DocumentOption::Landscape(options.landscape),
            DocumentOption::BulletMarker(options.bullet_marker.clone()),
        ] {
            if let Err(error) = self.check_option(&SetOptionParams {
                guard: p.guard,
                option,
            }) {
                diagnostics.push(Diagnostic::error(error.reason(), &error));
            }
        }
        for field in self.metadata().field_values() {
            if field.required_for_export && field.value.trim().is_empty() {
                diagnostics.push(Diagnostic {
                    severity: Severity::Warning,
                    code: "export_required_field".into(),
                    detail: None,
                    field: Some(field.field),
                });
            }
        }
        let valid = !diagnostics.iter().any(|d| d.severity == Severity::Error);
        Ok(ValidationResult {
            guard: p.guard,
            valid,
            ready_for_export: valid && self.metadata().ready_for_export(),
            diagnostics,
        })
    }
}
