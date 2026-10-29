//! MCP permissions and document references over the common structured session.
use super::{roots::Roots, types::*};
use serde_json::{Map, Value, json};
use typstographe_core::{
    content::api::{self, Session},
    protocol::{CommandError, ErrorCode},
};
pub struct Host {
    pub session: Session,
    roots: Roots,
    read_only: bool,
}
impl Host {
    pub fn new(
        read: Vec<std::path::PathBuf>,
        write: Vec<std::path::PathBuf>,
        read_only: bool,
    ) -> Result<Self, CommandError> {
        Ok(Self {
            session: Session::new(),
            roots: Roots::new(read, write)?,
            read_only,
        })
    }
    pub fn permissions(&self) -> Value {
        let mut p = self.roots.describe();
        p["read_only"] = json!(self.read_only);
        p["direct_edit_enabled"] = json!(!self.read_only);
        p["read_enabled"] = json!(self.roots.can_read());
        p["write_enabled"] = json!(!self.read_only && self.roots.can_write());
        p["human_approval_required"] = json!(false);
        p
    }
    fn reference(&self) -> Value {
        self.session
            .guard()
            .map(|g| json!({"id":g.document,"guard":g}))
            .unwrap_or(Value::Null)
    }
    fn check_reference(
        &self,
        name: &str,
        params: &mut Map<String, Value>,
    ) -> Result<(), CommandError> {
        let reference = params
            .remove("document")
            .map(api::decode::<DocumentRef>)
            .transpose()?;
        let global = matches!(
            name,
            "document.inspect"
                | "document_types.list"
                | "document_types.describe"
                | "commands.list"
                | "capabilities.describe"
                | "spelling.suggestions"
        );
        if let Some(reference) = reference {
            let guard = self
                .session
                .guard()
                .ok_or_else(|| CommandError::new(ErrorCode::Unavailable, "no_document"))?;
            if reference.id != guard.document {
                return Err(CommandError::new(
                    ErrorCode::Document,
                    "mcp_document_foreign",
                ));
            }
            reference.guard.check(guard).map_err(api::content_error)?;
        } else if !global
            && (!matches!(name, "document.new" | "document.open") || self.session.is_open())
        {
            return Err(CommandError::new(
                ErrorCode::InvalidParams,
                "mcp_document_required",
            ));
        }
        Ok(())
    }
    fn path(value: &Value, key: &str) -> Result<std::path::PathBuf, CommandError> {
        api::decode(
            value
                .get(key)
                .cloned()
                .ok_or_else(|| api::content_error(api::invalid(key)))?,
        )
    }
    fn authorize(&self, name: &str, p: &mut Value) -> Result<(), CommandError> {
        if self.read_only
            && !api::is_query(name)
            && !matches!(name, "document.close" | "document.open")
        {
            return Err(CommandError::new(ErrorCode::Unavailable, "mcp_read_only"));
        }
        match name {
            "document.open" => {
                let read_only = p["read_only"].as_bool().unwrap_or(false);
                if self.read_only && !read_only {
                    return Err(CommandError::new(ErrorCode::Unavailable, "mcp_read_only"));
                }
                let path = self.roots.read_file(&Self::path(p, "path")?)?;
                p["path"] = json!(if read_only {
                    path
                } else {
                    self.roots.document_write(&path)?
                });
            }
            "document.save_as" => {
                p["path"] = json!(self.roots.document_write(&Self::path(p, "path")?)?)
            }
            "document.save" => {
                self.roots
                    .document_write(self.session.file_path().ok_or_else(|| {
                        CommandError::new(ErrorCode::InvalidParams, "save_path_required")
                    })?)?;
            }
            "document.export" => {
                p["directory"] = json!(self.roots.write(&Self::path(p, "directory")?)?)
            }
            "document.batch" => {
                if let Some(save) = p.get_mut("save") {
                    match save["kind"].as_str() {
                        Some("as") => {
                            save["path"] =
                                json!(self.roots.document_write(&Self::path(save, "path")?)?)
                        }
                        Some("current") => {
                            self.roots
                                .document_write(self.session.file_path().ok_or_else(|| {
                                    CommandError::new(
                                        ErrorCode::InvalidParams,
                                        "save_path_required",
                                    )
                                })?)?;
                        }
                        _ => return Err(api::content_error(api::invalid("save"))),
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    pub fn dispatch(
        &mut self,
        name: &str,
        mut arguments: Map<String, Value>,
    ) -> Result<Reply, CommandError> {
        if serde_json::to_vec(&arguments).unwrap().len()
            > typstographe_core::content::shared::MAX_STATE_BYTES
        {
            return Err(CommandError::new(
                ErrorCode::InvalidParams,
                "batch_size_limit",
            ));
        }
        super::validation::check(false, name, &json!(arguments))?;
        self.check_reference(name, &mut arguments)?;
        let mut params = json!(arguments);
        self.authorize(name, &mut params)?;
        let png = if name == "render.page" {
            let page = params["page"]
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| api::content_error(api::invalid("page")))?;
            api::check_params(name, &params)?;
            Some(self.session.page_png(page)?)
        } else {
            None
        };
        let outcome = if name == "render.page" {
            json!({"guard":self.session.guard(),"page":params["page"]})
        } else if api::is_query(name) {
            self.session.query(name, params)?
        } else {
            self.session.execute(name, params)?
        };
        Ok(Reply {
            value: json!({"version":1,"ok":true,"document":self.reference(),"outcome":outcome,"summary":typstographe_i18n::tr!("mcp-completed",id=name),"permissions":self.permissions()}),
            png,
        })
    }
    pub fn resource(&mut self, uri: &str) -> Result<Reply, CommandError> {
        let outcome = match uri {
            "typstographe://guide" => {
                json!({"instructions":super::instructions(false),"resources":super::resources::list()})
            }
            "typstographe://types" => api::types(),
            "typstographe://commands" => api::catalogue(),
            "typstographe://content-capabilities" => {
                json!({"entries":typstographe_core::content::catalogue::ALL,"commands":api::catalogue()})
            }
            "typstographe://document" => {
                if self.session.is_open() {
                    json!({"inspect":self.session.query("document.inspect",json!({}))?,"structure":self.session.query("document.structure",json!({}))?,"metadata":self.session.query("metadata.get",json!({}))?})
                } else {
                    json!({"open":false})
                }
            }
            "typstographe://models" => {
                if self.session.is_open() {
                    self.session.query("models.list", json!({}))?
                } else {
                    api::types()
                }
            }
            _ => {
                return Err(CommandError::new(
                    ErrorCode::NotFound,
                    "mcp_resource_unknown",
                ));
            }
        };
        Ok(Reply {
            value: json!({"version":1,"document":self.reference(),"permissions":self.permissions(),"outcome":outcome}),
            png: None,
        })
    }
}
