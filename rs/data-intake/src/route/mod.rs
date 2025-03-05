mod intake_customresource;
mod intake_event;
mod intake_log;
mod intake_resource;

pub use intake_customresource::{customresource_intake, customresources_intake};
pub use intake_event::{event_intake, events_intake};
pub use intake_log::log_intake;
pub use intake_resource::{resource_intake, resources_intake};
