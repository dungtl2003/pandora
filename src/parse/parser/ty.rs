use crate::ast::{Ty, TyKind};

use super::{path::PathStyle, PResult, Parser};

impl Parser<'_> {
    pub fn parse_ty(&mut self) -> PResult<Box<Ty>> {
        let path = self.parse_path(PathStyle::Type)?;
        let span = path.span;
        let kind = TyKind::Path(path);
        Ok(Box::new(Ty { kind, span }))
    }
}
