use handlebars::{
    BlockContext, Context, Handlebars, Helper, HelperDef, HelperResult, Output, RenderContext,
    RenderError, Renderable,
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
///     handlebars.render_template(tpl, &json!({"access": "admin"})).unwrap(),
///     "Admin"
/// );
///
/// assert_eq!(
///     handlebars.render_template(tpl, &json!({"access": "nobody"})).unwrap(),
///     "User"
/// );
/// # }
///

#[derive(Clone, Copy)]
pub struct DefaultHelper;

impl HelperDef for DefaultHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'reg>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        if let Some(ref mut block) = rc.block_mut() {
            let prev_found = block
                .get_local_var("match")
                .and_then(Value::as_bool)
                .unwrap_or_default();
            if !prev_found {
                // fallback to default if no match was found
                match h.template() {
                    Some(ref t) => t.render(r, ctx, rc, out),
                    None => Ok(()),
                }
            } else {
                // skip if found match already
                Ok(())
            }
        } else {
            Ok(())
        }
    }
}

#[derive(Clone)]
pub struct CaseHelper {
    expression_value: serde_json::Value,
}

impl HelperDef for CaseHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'reg>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        if let Some(ref mut block) = rc.block_mut() {
            let prev_found = block
                .get_local_var("match")
                .and_then(Value::as_bool)
                .unwrap_or_default();
            if !prev_found
                && h.params()
                    .iter()
                    .any(|x| *x.value() == self.expression_value)
            {
                // found match
                block.set_local_var("@match".to_string(), json!(true));
                match h.template() {
                    Some(ref t) => t.render(r, ctx, rc, out),
                    None => Ok(()),
                }
            } else {
                // did not find match
                Ok(())
            }
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy)]
pub struct SwitchHelper;

impl HelperDef for SwitchHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'reg>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        // Read in the switch variable or expression
        let param = h
            .param(0)
            .ok_or_else(|| RenderError::new("Param not found for helper \"switch\""))?;

        let expression_value = param.value().clone();

        // Keep track of whether a match occurs within the block
        let mut block_context = BlockContext::new();
        block_context.set_local_var("@match".to_string(), json!(false));
        let mut local_rc = rc.clone();
        local_rc.push_block(block_context);

        // Add the `{{#case}}` helper within the `{{#switch}}` block
        local_rc.register_local_helper("case", Box::new(CaseHelper { expression_value }));

        // Add the `{{#default}}` helper within the `{{#switch}}` block
        local_rc.register_local_helper("default", Box::new(DefaultHelper));

        // Render the `{{#switch}}` block
        let result = match h.template() {
            Some(ref t) => t.render(r, ctx, &mut local_rc, out),
            None => Ok(()),
        };

        local_rc.pop_block();

        result
    }
}

#[cfg(test)]
mod tests {
    use super::SwitchHelper;
    use handlebars::Handlebars;

    #[test]
    fn test_switch() {
        let tpl = "\
            {{#switch state}}\
                {{#case \"page1\" \"page2\"}}\
                    page 1 or 2\
                    {{#switch s}}\
                        {{#case 4}}s = 4{{/case}}\
                    {{/switch}}\
                {{/case}}\
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

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));
        assert!(handlebars.register_template_string("tpl", tpl).is_ok());

        let r0 = handlebars.render("tpl", &json!({"state": "page2", "s": 1}));
        assert_eq!(r0.ok().unwrap(), "page 1 or 2");

        let r1 = handlebars.render("tpl", &json!({"state": "page5", "s": 1}));
        assert_eq!(r1.ok().unwrap(), "page5 - s = 1");

        let r2 = handlebars.render("tpl", &json!({"state": "page5", "s": 4}));
        assert_eq!(r2.ok().unwrap(), "page5 - unknown");

        let r3 = handlebars.render("tpl", &json!({"state": "page0", "s": 1}));
        assert_eq!(r3.ok().unwrap(), "page0");
    }

    #[test]
    fn test_missing_key_renders_default() {
        let tpl = "\
            {{#switch access}}\
                {{#case \"admin\"}}Admin{{/case}}\
                {{#default}}User{{/default}}\
            {{/switch}}\
        ";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));

        assert_eq!(handlebars.render_template(tpl, &json!({})).unwrap(), "User");
    }

    #[test]
    fn test_case_helper_not_defined() {
        let tpl = "\
            {{#switch access}}\
                {{#case \"admin\"}}Admin{{/case}}\
                {{#default}}User{{/default}}\
            {{/switch}}\
            {{#case \"test\"}}Check{{/case}}\
        ";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));

        assert!(handlebars
            .render_template(tpl, &json!({"access": "admin"}))
            .is_err());
    }

    #[test]
    fn test_default_helper_not_defined() {
        let tpl = "\
            {{#switch access}}\
                {{#case \"admin\"}}Admin{{/case}}\
                {{#default}}User{{/default}}\
            {{/switch}}\
            {{#default \"test\"}}Check{{/default}}\
        ";

        let mut handlebars = Handlebars::new();
        handlebars.register_helper("switch", Box::new(SwitchHelper));

        assert!(handlebars
            .render_template(tpl, &json!({"access": "admin"}))
            .is_err());
    }
}
