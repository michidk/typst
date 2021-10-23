use super::*;
use crate::layout::{
    GridNode, PadNode, ShapeKind, ShapeNode, StackChild, StackNode, TrackSizing,
};
use crate::style::{Paper, PaperClass};

/// `page`: Configure pages.
pub fn page(ctx: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let paper = match args.named::<Spanned<Str>>("paper")?.or_else(|| args.eat()) {
        Some(name) => match Paper::from_name(&name.v) {
            None => bail!(name.span, "invalid paper name"),
            paper => paper,
        },
        None => None,
    };

    let width = args.named("width")?;
    let height = args.named("height")?;
    let margins = args.named("margins")?;
    let left = args.named("left")?;
    let top = args.named("top")?;
    let right = args.named("right")?;
    let bottom = args.named("bottom")?;
    let flip = args.named("flip")?;

    let page = ctx.style.page_mut();

    if let Some(paper) = paper {
        page.class = paper.class();
        page.size = paper.size();
    }

    if let Some(width) = width {
        page.class = PaperClass::Custom;
        page.size.w = width;
    }

    if let Some(height) = height {
        page.class = PaperClass::Custom;
        page.size.h = height;
    }

    if let Some(margins) = margins {
        page.margins = Sides::splat(Some(margins));
    }

    if let Some(left) = left {
        page.margins.left = Some(left);
    }

    if let Some(top) = top {
        page.margins.top = Some(top);
    }

    if let Some(right) = right {
        page.margins.right = Some(right);
    }

    if let Some(bottom) = bottom {
        page.margins.bottom = Some(bottom);
    }

    if flip.unwrap_or(false) {
        std::mem::swap(&mut page.size.w, &mut page.size.h);
    }

    Ok(Value::None)
}

/// `align`: Configure the alignment along the layouting axes.
pub fn align(ctx: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let first = args.eat::<Align>();
    let second = args.eat::<Align>();

    let mut horizontal = args.named("horizontal")?;
    let mut vertical = args.named("vertical")?;

    for value in first.into_iter().chain(second) {
        match value.axis() {
            Some(SpecAxis::Horizontal) | None if horizontal.is_none() => {
                horizontal = Some(value);
            }
            Some(SpecAxis::Vertical) | None if vertical.is_none() => {
                vertical = Some(value);
            }
            _ => {}
        }
    }

    if let Some(horizontal) = horizontal {
        ctx.style.text_mut().align = horizontal;
    }

    if let Some(vertical) = vertical {
        ctx.style.par_mut().align = vertical;
    }

    Ok(Value::None)
}

/// `linebreak`: Start a new line.
pub fn linebreak(_: &mut EvalContext, _: &mut Args) -> TypResult<Value> {
    Ok(Value::Node(Node::Linebreak))
}

/// `parbreak`: Start a new paragraph.
pub fn parbreak(_: &mut EvalContext, _: &mut Args) -> TypResult<Value> {
    Ok(Value::Node(Node::Parbreak))
}

/// `pagebreak`: Start a new page.
pub fn pagebreak(_: &mut EvalContext, _: &mut Args) -> TypResult<Value> {
    Ok(Value::Node(Node::Pagebreak))
}

/// `h`: Horizontal spacing.
pub fn h(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let spacing = args.expect("spacing")?;
    Ok(Value::Node(Node::Spacing(GenAxis::Inline, spacing)))
}

/// `v`: Vertical spacing.
pub fn v(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let spacing = args.expect("spacing")?;
    Ok(Value::Node(Node::Spacing(GenAxis::Block, spacing)))
}

/// `box`: Place content in a rectangular box.
pub fn boxed(ctx: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let width = args.named("width")?;
    let height = args.named("height")?;
    let fill = args.named("fill")?;
    let body: Node = args.eat().unwrap_or_default();
    Ok(Value::inline(ShapeNode {
        shape: ShapeKind::Rect,
        width,
        height,
        fill: fill.map(Paint::Color),
        child: Some(body.to_block(&ctx.style)),
    }))
}

/// `block`: Place content in a block.
pub fn block(ctx: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let body: Node = args.expect("body")?;
    Ok(Value::block(body.to_block(&ctx.style)))
}

/// `pad`: Pad content at the sides.
pub fn pad(ctx: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let all = args.eat();
    let left = args.named("left")?;
    let top = args.named("top")?;
    let right = args.named("right")?;
    let bottom = args.named("bottom")?;
    let body: Node = args.expect("body")?;

    let padding = Sides::new(
        left.or(all).unwrap_or_default(),
        top.or(all).unwrap_or_default(),
        right.or(all).unwrap_or_default(),
        bottom.or(all).unwrap_or_default(),
    );

    Ok(Value::block(PadNode {
        padding,
        child: body.to_block(&ctx.style),
    }))
}

/// `stack`: Stack children along an axis.
pub fn stack(ctx: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    enum Child {
        Spacing(Linear),
        Any(Node),
    }

    castable! {
        Child: "linear or template",
        Value::Length(v) => Self::Spacing(v.into()),
        Value::Relative(v) => Self::Spacing(v.into()),
        Value::Linear(v) => Self::Spacing(v),
        Value::Node(v) => Self::Any(v),
    }

    let dir = args.named("dir")?.unwrap_or(Dir::TTB);
    let spacing = args.named::<Linear>("spacing")?;

    let mut children = vec![];
    let mut delayed = None;

    // Build the list of stack children.
    for child in args.all() {
        match child {
            Child::Spacing(v) => {
                children.push(StackChild::Spacing(v));
                delayed = None;
            }
            Child::Any(template) => {
                if let Some(v) = delayed {
                    children.push(StackChild::Spacing(v));
                }

                let node = template.to_block(&ctx.style);
                children.push(StackChild::Any(node, ctx.style.par.align));
                delayed = spacing;
            }
        }
    }

    Ok(Value::block(StackNode { dir, children }))
}

/// `grid`: Arrange children into a grid.
pub fn grid(ctx: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    castable! {
        Vec<TrackSizing>: "integer or (auto, linear, fractional, or array thereof)",
        Value::Auto => vec![TrackSizing::Auto],
        Value::Length(v) => vec![TrackSizing::Linear(v.into())],
        Value::Relative(v) => vec![TrackSizing::Linear(v.into())],
        Value::Linear(v) => vec![TrackSizing::Linear(v)],
        Value::Fractional(v) => vec![TrackSizing::Fractional(v)],
        Value::Int(count) => vec![TrackSizing::Auto; count.max(0) as usize],
        Value::Array(values) => values
            .into_iter()
            .filter_map(|v| v.cast().ok())
            .collect(),
    }

    castable! {
        TrackSizing: "auto, linear, or fractional",
        Value::Auto => Self::Auto,
        Value::Length(v) => Self::Linear(v.into()),
        Value::Relative(v) => Self::Linear(v.into()),
        Value::Linear(v) => Self::Linear(v),
        Value::Fractional(v) => Self::Fractional(v),
    }

    let columns = args.named("columns")?.unwrap_or_default();
    let rows = args.named("rows")?.unwrap_or_default();
    let tracks = Spec::new(columns, rows);

    let base_gutter: Vec<TrackSizing> = args.named("gutter")?.unwrap_or_default();
    let column_gutter = args.named("column-gutter")?;
    let row_gutter = args.named("row-gutter")?;
    let gutter = Spec::new(
        column_gutter.unwrap_or_else(|| base_gutter.clone()),
        row_gutter.unwrap_or(base_gutter),
    );

    let children = args.all().map(|node: Node| node.to_block(&ctx.style)).collect();

    Ok(Value::block(GridNode { tracks, gutter, children }))
}
