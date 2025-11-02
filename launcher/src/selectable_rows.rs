use iced::{
    Alignment, Element, Length, Padding, Size,
    advanced::{
        Layout, Shell, Widget,
        layout::{self, Limits, Node},
        renderer,
        widget::Tree,
    },
    keyboard::{Key, Modifiers, key::Named},
    mouse,
};

#[derive(Default, Debug, PartialEq, Eq)]
struct State {
    /// The index of the first row to draw
    first: usize,
    /// The index of the last row to draw
    last: usize,
    /// The index of the selected row
    selected: usize,
}

pub struct SelectableRows<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    width: Length,
    height: Length,
    padding: Padding,
    spacing: f32,
    align: Alignment,

    rows: Vec<Element<'a, Message, Theme, Renderer>>,
    row_min_height: f32,

    on_select: Box<dyn Fn(usize) -> Message + 'a>,
}

impl<'a, Message, Theme, Renderer> SelectableRows<'a, Message, Theme, Renderer> {
    pub fn with_rows(
        rows: impl IntoIterator<Item = Element<'a, Message, Theme, Renderer>>,
        row_min_height: f32,
        on_select: Box<dyn Fn(usize) -> Message + 'a>,
    ) -> Self {
        let rows = rows.into_iter().collect();
        Self {
            width: Length::Fill,
            height: Length::Shrink,
            padding: 0.into(),
            spacing: 0.0,
            align: Alignment::Start,
            rows,
            row_min_height,
            on_select,
        }
    }

    fn stateful_children(&self, state: &State) -> &[Element<'a, Message, Theme, Renderer>] {
        &self.rows[state.first..=state.last]
    }

    fn stateful_children_mut(
        &mut self,
        state: &State,
    ) -> &mut [Element<'a, Message, Theme, Renderer>] {
        &mut self.rows[state.first..=state.last]
    }
}

impl<'a, Message, Theme, Renderer: iced::advanced::Renderer> Widget<Message, Theme, Renderer>
    for SelectableRows<'a, Message, Theme, Renderer>
{
    fn size(&self) -> iced::Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let limits = limits
            .max_width(f32::INFINITY)
            .width(self.width)
            .height(f32::INFINITY);

        let state = tree.state.downcast_ref::<State>();

        layout::flex::resolve(
            layout::flex::Axis::Vertical,
            renderer,
            &limits,
            self.width,
            self.height,
            self.padding,
            self.spacing,
            self.align,
            self.stateful_children_mut(state),
            &mut tree.children,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();

        self.stateful_children(state)
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .for_each(|((child, state), layout)| {
                child
                    .as_widget()
                    .draw(state, renderer, theme, style, layout, cursor, viewport)
            })
    }

    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        iced::advanced::widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        iced::advanced::widget::tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        Vec::new()
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_ref::<State>();
        tree.diff_children(self.stateful_children(state));
    }

    fn operate(
        &mut self,
        _state: &mut Tree,
        _layout: Layout<'_>,
        _renderer: &Renderer,
        _operation: &mut dyn iced::advanced::widget::Operation,
    ) {
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &iced::Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let mut selected = state.selected;
        if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) =
            event
        {
            if *modifiers == Modifiers::empty()
                && (*key == Key::Named(Named::ArrowDown) || *key == Key::Named(Named::Tab))
            {
                selected += 1;
            } else if *key == Key::Named(Named::ArrowUp)
                || (*modifiers == Modifiers::SHIFT && *key == Key::Named(Named::Tab))
            {
                selected = selected.saturating_sub(1);
            }
        }

        // The theoretical max rows based on provided row height lower bound. This is the number of
        // rows that will be added to the widget tree as children.
        let max_visible_rows = (layout.bounds().height / self.row_min_height).ceil() as usize;

        // 0 <= first <= selected <= last <= rows.len() - 1
        let selected = selected.clamp(0, self.rows.len().saturating_sub(1));
        let mut first = state.first.clamp(0, selected);
        let mut last =
            selected.max((first + max_visible_rows - 1).min(self.rows.len().saturating_sub(1)));

        let Some(last_visible) = layout
            .children()
            .enumerate()
            .take_while(|(_, child_layout)| {
                layout.bounds().y + layout.bounds().height
                    > child_layout.bounds().y + child_layout.bounds().height
            })
            .map(|(i, _)| i)
            .last()
        else {
            // No children, nothing to do
            return;
        };

        // Scroll down so the selected row is above the last visible row, until the last visible
        // row is the last possible row
        let selected_row = selected - first;
        if first + last_visible + 1 < self.rows.len() && selected_row >= last_visible {
            let delta = selected_row - last_visible + 1;
            first = (first + delta).min(selected);
            last = (last + delta).min(self.rows.len().saturating_sub(1));
        }

        let new_state = State {
            selected,
            first,
            last,
        };

        if selected != state.selected || *state == State::default() {
            shell.publish((self.on_select)(selected));
        }

        tree.state = iced::advanced::widget::tree::State::new(new_state);
    }

    fn mouse_interaction(
        &self,
        _state: &Tree,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &iced::Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::None
    }
}

impl<'a, Message, Theme, Renderer> From<SelectableRows<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(selectable_rows: SelectableRows<'a, Message, Theme, Renderer>) -> Self {
        Self::new(selectable_rows)
    }
}
