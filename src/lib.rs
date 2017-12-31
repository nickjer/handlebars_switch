//! # Handlebars Switch Helper
//!
//! This provides a [Handlebars](http://handlebarsjs.com/) `{{#switch}}` helper to
//! the already incredible [handlebars-rust](https://github.com/sunng87/handlebars-rust)
//! crate.
//!
//! Links of interest:
//!
//! - [Documentation](https://docs.rs/handlebars_switch)
//! - [handlebars-rust](https://github.com/sunng87/handlebars-rust)
//! - [Handlebars](http://handlebarsjs.com)
//!
//! ## Quick Start
//!
//! You can easily add the ``{{#switch}}`` helper to a rust Handlebars object using
//! the `Handlebars#register_helper` method:
//!
//! ```ignore
//! use handlebars::Handlebars;
//! use handlebars_switch::Handlebars;
//!
//! let mut handlebars = Handlebars::new();
//! handlebars.register_helper("switch", Box::new(SwitchHelper));
//! ```
//!
//! ### Example
//!
//! Below is an example that renders a different page depending on the user's
//! access level:
//!
//!
//! ```
//! extern crate handlebars_switch;
//! extern crate handlebars;
//! #[macro_use] extern crate serde_json;
//!
//! use handlebars::Handlebars;
//! use handlebars_switch::SwitchHelper;
//!
//! fn main() {
//!   let mut handlebars = Handlebars::new();
//!   handlebars.register_helper("switch", Box::new(SwitchHelper));
//!
//!   let tpl = "\
//!       {{#switch access}}\
//!           {{#case \"admin\"}}Admin{{/case}}\
//!           {{#default}}User{{/default}}\
//!       {{/switch}}\
//!   ";
//!
//!   assert_eq!(
//!       handlebars.template_render(tpl, &json!({"access": "admin"})).unwrap(),
//!       "Admin"
//!   );
//!
//!   assert_eq!(
//!       handlebars.template_render(tpl, &json!({"access": "nobody"})).unwrap(),
//!       "User"
//!   );
//! }
//! ```

extern crate handlebars;
#[macro_use]
extern crate serde_json;

pub use self::switch::SwitchHelper;

mod switch;
