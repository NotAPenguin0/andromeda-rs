use egui::{
    Align, Align2, Context, FontSelection, InnerResponse, NumExt, Response, RichText,
    SelectableLabel, Sense, TextStyle, Ui, Vec2, Widget, WidgetText,
};

pub struct ToolButton<T> {
    label: String,
    tool: T,
}

fn make_rich_text(text: impl Into<String>, size: f32) -> WidgetText {
    RichText::new(text).size(size).into()
}

fn show_button(ui: &mut Ui, label: impl Into<String>, active: bool, size: f32) -> Response {
    let label = make_rich_text(label, size);
    let text = label.into_galley(ui, None, ui.available_width(), TextStyle::Button);

    let mut desired_size = Vec2::splat(size);
    desired_size.y = desired_size.y.at_least(ui.spacing().interact_size.y);
    let (rect, response) = ui.allocate_at_least(desired_size, Sense::click());

    if ui.is_rect_visible(response.rect) {
        let text_pos = ui.layout().align_size_within_rect(text.size(), rect).min;

        let visuals = ui.style().interact_selectable(&response, active);
        let rounding = rect.height() * 0.2;
        let rect = rect.expand(visuals.expansion);
        if active || response.hovered() || response.highlighted() || response.has_focus() {
            ui.painter()
                .rect(rect, rounding, visuals.weak_bg_fill, visuals.bg_stroke);
        } else {
            let fill = ui.style().visuals.widgets.hovered;
            ui.painter()
                .rect(rect, rounding, fill.bg_fill, fill.bg_stroke);
        }
        text.paint_with_visuals(ui.painter(), text_pos, &visuals);
    }
    response
}

impl<T> ToolButton<T> {
    fn show(self, ui: &mut Ui, active: bool, size: f32) -> InnerResponse<T> {
        let response = show_button(ui, self.label, active, size);
        InnerResponse {
            inner: self.tool,
            response,
        }
    }
}

impl<T> ToolButton<T> {
    pub fn new(label: impl Into<String>, tool: T) -> Self {
        Self {
            label: label.into(),
            tool,
        }
    }
}

pub struct Toolbar<'t, T> {
    active: &'t mut Option<T>,
    tools: Vec<ToolButton<T>>,
    size: f32,
}

impl<'t, T> Toolbar<'t, T> {
    fn is_active(active: &Option<T>, tool: &ToolButton<T>) -> bool {
        // If nothing is active, this tool is definitely not the active one
        let Some(active) = active else { return false };
        std::mem::discriminant(active) == std::mem::discriminant(&tool.tool)
    }
}

impl<'t, T> Toolbar<'t, T> {
    pub fn new(active: &'t mut Option<T>) -> Self {
        Self {
            active,
            tools: vec![],
            size: 24.0,
        }
    }

    pub fn tool(mut self, label: impl Into<String>, tool: T) -> Self {
        let button = ToolButton::new(label, tool);
        self.tools.push(button);
        self
    }

    pub fn show(self, ui: &mut Ui) {
        ui.vertical(move |ui| {
            for tool in self.tools {
                let active = Self::is_active(&self.active, &tool);
                let response = tool.show(ui, active, self.size);
                if response.response.clicked() {
                    *self.active = Some(response.inner);
                }
            }
            if show_button(ui, "🚫", false, self.size).clicked() {
                *self.active = None;
            }
        });
    }
}
