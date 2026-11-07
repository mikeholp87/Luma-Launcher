//! Portal requests subscribe before calling, so immediate responses are not lost.
use gio::prelude::*;
use glib::variant::ObjectPath;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};
const DEST: &str = "org.freedesktop.portal.Desktop";
const PATH: &str = "/org/freedesktop/portal/desktop";
const IFACE: &str = "org.freedesktop.portal.GlobalShortcuts";
type Options = HashMap<String, glib::Variant>;
fn request(
    bus: &gio::DBusConnection,
    method: &str,
    params: glib::Variant,
    token: &str,
    done: impl FnOnce(Result<Options, String>) + 'static,
) {
    let sender = bus
        .unique_name()
        .unwrap_or_default()
        .trim_start_matches(':')
        .replace('.', "_");
    let path = format!("/org/freedesktop/portal/desktop/request/{sender}/{token}");
    let callback = Rc::new(RefCell::new(Some(done)));
    let cb = callback.clone();
    let subscription = Rc::new(RefCell::new(None));
    let sub = subscription.clone();
    let timeout = Rc::new(RefCell::new(None::<glib::SourceId>));
    let response_timeout = timeout.clone();
    let id = bus.subscribe_to_signal(
        Some(DEST),
        Some("org.freedesktop.portal.Request"),
        Some("Response"),
        Some(&path),
        None,
        gio::DBusSignalFlags::NONE,
        move |signal| {
            let value = signal.parameters;
            if let Some(id) = response_timeout.borrow_mut().take() {
                id.remove();
            }
            // Dropping the subscription unsubscribes.
            sub.borrow_mut().take();
            if let Some(done) = cb.borrow_mut().take() {
                let result = value
                    .get::<(u32, Options)>()
                    .ok_or_else(|| "Malformed portal response".into())
                    .and_then(|(code, opts)| {
                        if code == 0 {
                            Ok(opts)
                        } else {
                            Err(format!("Shortcut request declined (code {code})"))
                        }
                    });
                done(result);
            }
        },
    );
    *subscription.borrow_mut() = Some(id);
    let timeout_bus = bus.clone();
    let timeout_callback = callback.clone();
    let timeout_subscription = subscription.clone();
    let timeout_holder = timeout.clone();
    let timeout_path = path.clone();
    *timeout.borrow_mut() = Some(glib::timeout_add_local_once(
        std::time::Duration::from_secs(120),
        move || {
            timeout_holder.borrow_mut().take();
            timeout_subscription.borrow_mut().take();
            timeout_bus.call(
                Some(DEST),
                &timeout_path,
                "org.freedesktop.portal.Request",
                "Close",
                None,
                None,
                gio::DBusCallFlags::NONE,
                1000,
                None::<&gio::Cancellable>,
                |_| {},
            );
            if let Some(done) = timeout_callback.borrow_mut().take() {
                done(Err("Shortcut request timed out; retry from Settings".into()));
            }
        },
    ));
    bus.call(
        Some(DEST),
        PATH,
        IFACE,
        method,
        Some(&params),
        None,
        gio::DBusCallFlags::NONE,
        10000,
        None::<&gio::Cancellable>,
        move |result| {
            if let Err(e) = result {
                if let Some(id) = timeout.borrow_mut().take() {
                    id.remove();
                }
                subscription.borrow_mut().take();
                if let Some(done) = callback.borrow_mut().take() {
                    done(Err(e.to_string()));
                }
            }
        },
    );
}
fn session_handle(options: &Options) -> Result<ObjectPath, String> {
    let value = options
        .get("session_handle")
        .and_then(|v| v.get::<String>())
        .ok_or("Portal did not return a string session handle")?;
    ObjectPath::try_from(value).map_err(|_| "Portal returned an invalid session path".into())
}
fn toggle_bound(options: &Options) -> bool {
    options
        .get("shortcuts")
        .and_then(|v| v.get::<Vec<(String, Options)>>())
        .is_some_and(|items| items.iter().any(|(id, _)| id == "toggle"))
}
fn activation(value: &glib::Variant, session: &str) -> Option<Option<String>> {
    let (path, id, _, options) = value.get::<(ObjectPath, String, u64, Options)>()?;
    (path.as_str() == session && id == "toggle").then(|| {
        options
            .get("activation_token")
            .and_then(|v| v.get::<String>())
    })
}
fn token() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "luma{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}
/// Owns portal session and subscriptions. Dropping it releases both.
pub struct Session {
    bus: gio::DBusConnection,
    path: ObjectPath,
    subscriptions: Vec<gio::SignalSubscription>,
    closed: Rc<Cell<bool>>,
}
impl Drop for Session {
    fn drop(&mut self) {
        self.subscriptions.clear();
        if self.closed.replace(true) {
            return;
        }
        self.bus.call(
            Some(DEST),
            self.path.as_str(),
            "org.freedesktop.portal.Session",
            "Close",
            None,
            None,
            gio::DBusCallFlags::NONE,
            1000,
            None::<&gio::Cancellable>,
            |_| {},
        );
    }
}
pub fn bind(
    trigger: &str,
    on_toggle: impl Fn(Option<String>) + 'static,
    on_closed: impl Fn() + 'static,
    on_result: impl FnOnce(Result<Session, String>) + 'static,
) {
    let trigger = trigger.to_owned();
    gio::bus_get(
        gio::BusType::Session,
        None::<&gio::Cancellable>,
        move |result| {
            let bus = match result {
                Ok(bus) => bus,
                Err(e) => {
                    on_result(Err(e.to_string()));
                    return;
                }
            };
            let request_token = token();
            let mut options = Options::new();
            options.insert("handle_token".into(), request_token.to_variant());
            options.insert("session_handle_token".into(), token().to_variant());
            let connection = bus.clone();
            request(
                &bus,
                "CreateSession",
                (options,).to_variant(),
                &request_token,
                move |result| {
                    let path = match result.and_then(|options| session_handle(&options)) {
                        Ok(path) => path,
                        Err(e) => {
                            on_result(Err(e));
                            return;
                        }
                    };
                    let closed = Rc::new(Cell::new(false));
                    let mut session = Session {
                        bus: connection.clone(),
                        path: path.clone(),
                        subscriptions: vec![],
                        closed: closed.clone(),
                    };
                    let request_token = token();
                    let mut props = Options::new();
                    props.insert("description".into(), "Toggle Luma Launcher".to_variant());
                    props.insert("preferred_trigger".into(), trigger.to_variant());
                    let shortcuts = vec![("toggle".to_owned(), props)];
                    let mut options = Options::new();
                    options.insert("handle_token".into(), request_token.to_variant());
                    let activated_path = path.to_string();
                    session.subscriptions.push(connection.subscribe_to_signal(
                        Some(DEST),
                        Some(IFACE),
                        Some("Activated"),
                        Some(PATH),
                        None,
                        gio::DBusSignalFlags::NONE,
                        move |signal| {
                            if let Some(token) = activation(signal.parameters, &activated_path) {
                                on_toggle(token);
                            }
                        },
                    ));
                    let report: Rc<dyn Fn()> = Rc::new(on_closed);
                    let session_closed = closed.clone();
                    let session_report = report.clone();
                    session.subscriptions.push(connection.subscribe_to_signal(
                        Some(DEST),
                        Some("org.freedesktop.portal.Session"),
                        Some("Closed"),
                        Some(path.as_str()),
                        None,
                        gio::DBusSignalFlags::NONE,
                        move |_| {
                            if !session_closed.replace(true) {
                                session_report();
                            }
                        },
                    ));
                    let service_closed = closed.clone();
                    session.subscriptions.push(connection.subscribe_to_signal(
                        Some("org.freedesktop.DBus"),
                        Some("org.freedesktop.DBus"),
                        Some("NameOwnerChanged"),
                        Some("/org/freedesktop/DBus"),
                        Some(DEST),
                        gio::DBusSignalFlags::NONE,
                        move |signal| {
                            if let Some((_, old, new)) =
                                signal.parameters.get::<(String, String, String)>()
                            {
                                if !old.is_empty() && old != new && !service_closed.replace(true) {
                                    report();
                                }
                            }
                        },
                    ));
                    request(
                        &connection,
                        "BindShortcuts",
                        (path, shortcuts, "", options).to_variant(),
                        &request_token,
                        move |result| match result {
                            Ok(options) if !closed.get() && toggle_bound(&options) => {
                                on_result(Ok(session))
                            }
                            Ok(_) => on_result(Err("The portal granted no toggle shortcut".into())),
                            Err(e) => on_result(Err(e)),
                        },
                    );
                },
            );
        },
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portal_string_session_handle() {
        let mut options = Options::new();
        options.insert(
            "session_handle".into(),
            "/org/freedesktop/portal/desktop/session/test/luma".to_variant(),
        );
        assert!(session_handle(&options).is_ok());
        options.insert("session_handle".into(), "not a path".to_variant());
        assert!(session_handle(&options).is_err());
    }
    #[test]
    fn empty_grants_are_not_success() {
        let mut options = Options::new();
        assert!(!toggle_bound(&options));
        options.insert(
            "shortcuts".into(),
            vec![("toggle".to_owned(), Options::new())].to_variant(),
        );
        assert!(toggle_bound(&options));
    }
    #[test]
    fn malformed_activation_is_ignored() {
        assert!(activation(&"bad".to_variant(), "/test").is_none());
        let path = ObjectPath::try_from("/test").unwrap();
        let value = (path, "toggle", 0_u64, Options::new()).to_variant();
        assert_eq!(activation(&value, "/test"), Some(None));
        assert!(activation(&value, "/other").is_none());
    }
    #[test]
    fn request_tokens_are_unique() {
        assert_ne!(token(), token());
    }
}
