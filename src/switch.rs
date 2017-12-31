use handlebars::{
    HelperDef, Helper, Handlebars, RenderContext, Renderable, RenderError
};

use serde_json::Value;

/// Switch Helper
///
/// Provides the `{{#switch}}` helper to a Handlebars template.
///
/// # Examples
///
/// ```
/// # extern crate handlebars_switch;
/// # extern crate handlebars;
/// # #[macro_use] extern crate serde_json;
/// # fn main() {
/// use handlebars::Handlebars;
/// use handlebars_switch::SwitchHelper;
///
/// let mut handlebars = Handlebars::new();
/// handlebars.register_helper("switch", Box::new(SwitchHelper));
///
/// let tpl = "\
///     {{#switch access}}\
///         {{#case \"admin\"}}Admin{{/case}}\
///         {{#default}}User{{/default}}\
///     {{/switch}}\
/// ";
///
/// assert_eq!(
///     handlebars.template_render(tpl, &json!({"access": "admin"})).unwrap(),
///     "Admin"
/// );
///
/// assert_eq!(
///     handlebars.template_render(tpl, &json!({"access": "nobody"})).unwrap(),
///     "User"
/// );
/// # }
///

#[derive(Clone, Copy)]
pub struct SwitchHelper;

impl HelperDef for SwitchHelper {
    fn call(&self, h: &Helper, r: &Handlebars, rc: &mut RenderContext) -> Result<(), RenderError> {
        // Read in the switch variable or expression
        let param = try!(
            h.param(0).ok_or_else(|| { RenderError::new("Param not found for helper \"switch\"") })
        );
        let value = param.value().clone();

        // Keep track of whether a match occurs within the block
        let mut local_rc = rc.derive();
        local_rc.set_local_var("@match".to_string(), json!(false));

        // Add the `{{#case}}` helper within the `{{#switch}}` block
        local_rc.register_local_helper(
            "case",
            Box::new(move |h: &Helper, r: &Handlebars, rc: &mut RenderContext| {
                let prev_found = rc
                    .get_local_var(&String::from("@match"))
                    .and_then(Value::as_bool)
                    .unwrap_or_default();
                if !prev_found && h.params().iter().any(|x| x.value() == &value) {
                    // found match
                    rc.set_local_var("@match".to_string(), json!(true));
                    match h.template() {
                        Some(ref t) => t.render(r, rc),
                        None => Ok(()),
                    }
                } else {
                    // did not find match
                    Ok(())
                }
            }),
        );

        // Add the `{{#default}}` helper within the `{{#switch}}` block
        local_rc.register_local_helper(
            "default",
            Box::new(|h: &Helper, r: &Handlebars, rc: &mut RenderContext| {
                let prev_found = rc
                    .get_local_var(&String::from("@match"))
                    .and_then(Value::as_bool)
                    .unwrap_or_default();
                if !prev_found {
                    // fallback to default if no match was found
                    match h.template() {
                        Some(ref t) => t.render(r, rc),
                        None => Ok(()),
                    }
                } else {
                    // skip if found match already
                    Ok(())
                }
            }),
        );

        // Render the `{{#switch}}` block
        match h.template() {
            Some(ref t) => t.render(r, &mut local_rc),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use handlebars::Handlebars;
    use super::SwitchHelper;

    #[test]
    fn test_switch() {
        let tpl = "\
            {{#switch state}}\
                {{#case \"page1\" \"page2\"}}page 1 or 2{{/case}}\
                {{#case \"page3\"}}page3{{/case}}\
                {{#case \"page4\"}}page4{{/case}}\
                {{#case \"page5\"}}\
                    page5 - \
                    {{#switch s}}\
                        {{#case 3}}s = 3{{/case}}\
                        {{#case 2}}s = 2{{/case}}\
                        {{#case 1}}s = 1{{/case}}\
                        {{#default}}unknown{{/default}}\
                    {{/switch}}\
                {{/case}}\
                {{#default}}page0{{/default}}\
            {{/switch}}\
        ";
        let ctx0 = json!({
            "state": "page2",
            "s": 1
        });
        let ctx1 = json!({
            "state": "page5",
            "s": 1
        });
        let ctx2 = json!({
            "state": "page0",
            "s": 1
        });

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));
        assert!(
            handlebars.register_template_string("tpl", tpl).is_ok()
        );

        let r0 = handlebars.render("tpl", &ctx0);
        assert_eq!(r0.ok().unwrap(), "page 1 or 2");

        let r1 = handlebars.render("tpl", &ctx1);
        assert_eq!(r1.ok().unwrap(), "page5 - s = 1");

        let r2 = handlebars.render("tpl", &ctx2);
        assert_eq!(r2.ok().unwrap(), "page0");
    }
}
