use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{
    layout, mouse, overlay, renderer, Clipboard, Layout, Shell, Widget,
};
use iced::{Element, Length, Pixels, Rectangle, Size, Vector};

pub struct EqualHeightRow<
    'a,
    Message,
    Theme = iced::Theme,
    Renderer = iced::Renderer,
> {
    spacing: f32,
    max_item_width: f32,
    width: Length,
    height: Length,
    children: Vec<Element<'a, Message, Theme, Renderer>>,
}

impl<'a, Message, Theme, Renderer> EqualHeightRow<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    pub fn new(
        children: impl IntoIterator<Item = Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        Self {
            spacing: 0.0,
            max_item_width: f32::INFINITY,
            width: Length::Shrink,
            height: Length::Shrink,
            children: children.into_iter().collect(),
        }
    }

    pub fn spacing(mut self, amount: impl Into<Pixels>) -> Self {
        self.spacing = amount.into().0;
        self
    }

    pub fn max_item_width(mut self, width: f32) -> Self {
        self.max_item_width = width.max(0.0);
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for EqualHeightRow<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        if self.children.is_empty() {
            return layout::Node::new(
                limits.resolve(self.width, self.height, Size::ZERO),
            );
        }

        let limits = limits.width(self.width).height(self.height);
        let count = self.children.len();
        let spacing_total = self.spacing * (count.saturating_sub(1) as f32);

        let max_width = limits.max().width;
        let max_item_width = if self.max_item_width.is_finite() {
            self.max_item_width
        } else {
            max_width
        };

        let mut item_width = if max_width.is_finite() {
            let available = (max_width - spacing_total).max(0.0);
            let per_width = available / count as f32;
            per_width.min(max_item_width)
        } else {
            max_item_width
        };

        if !item_width.is_finite() {
            item_width = 0.0;
        }

        let max_height = limits.max().height;
        // Ignore Fill heights while measuring so intrinsic content drives max.
        let measure_limits = layout::Limits::with_compression(
            Size::new(item_width, 0.0),
            Size::new(item_width, max_height),
            Size::new(false, true),
        );

        let mut max_child_height: f32 = 0.0;
        for (child, state) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
        {
            let node =
                child.as_widget_mut().layout(state, renderer, &measure_limits);
            max_child_height = max_child_height.max(node.size().height);
        }

        let target_height = max_child_height.min(max_height);
        let forced_limits = layout::Limits::new(
            Size::new(item_width, target_height),
            Size::new(item_width, target_height),
        );

        let mut children = Vec::with_capacity(count);
        let mut x = 0.0;

        for (child, state) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
        {
            let node =
                child.as_widget_mut().layout(state, renderer, &forced_limits);
            children.push(node.move_to((x, 0.0)));
            x += item_width + self.spacing;
        }

        let intrinsic_size = Size::new(
            item_width * count as f32 + spacing_total,
            target_height,
        );

        let size = limits.resolve(self.width, self.height, intrinsic_size);

        layout::Node::with_children(size, children)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((child, state), layout)| {
                    child
                        .as_widget_mut()
                        .operate(state, layout, renderer, operation);
                });
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        for ((child, tree), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell,
                viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, tree), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if let Some(clipped_viewport) = layout.bounds().intersection(viewport) {
            for ((child, tree), layout) in self
                .children
                .iter()
                .zip(&tree.children)
                .zip(layout.children())
                .filter(|(_, layout)| layout.bounds().intersects(viewport))
            {
                child.as_widget().draw(
                    tree,
                    renderer,
                    theme,
                    style,
                    layout,
                    cursor,
                    &clipped_viewport,
                );
            }
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer>
    From<EqualHeightRow<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(row: EqualHeightRow<'a, Message, Theme, Renderer>) -> Self {
        Self::new(row)
    }
}
