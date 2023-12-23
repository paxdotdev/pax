


use crate::orm::PaxManifestORM;

pub struct DesigntimeApi<R> {
    orm: PaxManifestORM<R>,
    llm: (),
}