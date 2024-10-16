use std::{borrow::BorrowMut, cell::RefCell, collections::HashMap, ops::Deref, rc::Rc};

use pax_designtime::orm::manifest_modification_data::ManifestModificationData;
use pax_engine::{
    api::{properties::UntypedProperty, Store},
    log, Property,
};

impl Store for GranularManifestChangeStore {}

/// Added to the top level of the designer on mount,
/// to be used by designer components to listen to
/// granular changes to the manifest.
/// NOTE: some sub-components (such as property editors), such as property
/// editors, actively add new values to this store, to "ask to know about
/// changes" while others listen to properties already present without
/// performing modifications (such as tree view).
#[derive(Clone)]
pub struct GranularManifestChangeStore(Rc<NotificationData>);

// Rc wrapped inner data type
struct NotificationData {
    notify_node_property_changed: RefCell<HashMap<String, Property<()>>>,
    notify_tree_changed: Property<()>,
    // NOTE: Use this only if a more granular update isn't
    // available/worthwhile to punch through the ORM.
    something_changed: Property<()>,
}

impl Default for GranularManifestChangeStore {
    fn default() -> Self {
        Self(Rc::new(NotificationData {
            notify_node_property_changed: Default::default(),
            notify_tree_changed: Default::default(),
            something_changed: Default::default(),
        }))
    }
}

impl GranularManifestChangeStore {
    pub(crate) fn notify_from_manifest_modification_data(
        &self,
        manifest_updates: ManifestModificationData,
    ) {
        let ManifestModificationData {
            modified_properties,
            tree_modified,
        } = manifest_updates;
        for prop in modified_properties.iter() {
            if let Some(notifier) = self.0.notify_node_property_changed.borrow().get(prop) {
                notifier.set(());
                log::trace!(
                    "manifest change notification store: property name {:?} changed, and was listened to",
                    prop
                );
            } else {
                log::trace!("manifest change notification store: property name {:?} changed, was not listened to", prop);
            }
        }
        if tree_modified {
            self.0.notify_tree_changed.set(());
            log::trace!("manifest change notification store: tree changed");
        }
        self.0.something_changed.set(());
    }

    pub(crate) fn register_property_notifier(&self, name: &str) -> UntypedProperty {
        let notifier = Property::new(());
        let untyped = notifier.untyped();
        self.0
            .notify_node_property_changed
            .borrow_mut()
            .insert(name.to_owned(), notifier);
        untyped
    }

    pub(crate) fn remove_property_notifier(&self, name: &str) {
        self.0
            .notify_node_property_changed
            .borrow_mut()
            .remove(name);
    }

    pub(crate) fn get_tree_changed_notifier(&self) -> UntypedProperty {
        self.0.notify_tree_changed.untyped()
    }

    pub(crate) fn get_manifest_any_change_notifier(&self) -> UntypedProperty {
        self.0.something_changed.untyped()
    }
}
