//! The results list and its per-row rendering.
//!
//! The empty state (`no_results_view`) lives here too: deciding what the panel
//! shows is inseparable from building it.

use super::super::model::{ColumnFilterField, ResultsView, SortBy};
use super::super::{App, FileItem, Message, theme};
use super::snippet::parse_snippet;
use super::welcome::welcome_hero_view;
use crate::iced_ui::icons::load_icon_size;
use iced::widget::{Space, TextInput, button, column, container, mouse_area, row, scrollable, text};
use iced::{Alignment, Element, Font, Length, Padding, font};

#[allow(clippy::items_after_statements)]
#[allow(clippy::too_many_lines)]
pub(super) fn results_panel(app: &App) -> Element<'_, Message> {
    if app.results.is_empty() {
        if app.search_query.is_empty() {
            return welcome_hero_view(app);
        }
        return no_results_view(app);
    }

    let max_display = app.settings.results_per_page.clamp(25, 200);
    let is_table = app.results_view == ResultsView::Table;

    // Collect indices of results matching inline column filters (in Table mode).
    let visible_indices: Vec<usize> = if is_table {
        app.results
            .iter()
            .enumerate()
            .filter(|(_, r)| app.column_filters.matches(r))
            .map(|(i, _)| i)
            .collect()
    } else {
        (0..app.results.len()).collect()
    };

    // Virtual scrolling: only render items visible in the viewport plus a
    // small buffer.
    let item_height: f32 = if is_table { 32.0 } else { 90.0 };
    const VISIBLE_BUFFER: usize = 5;
    let viewport_height = 600.0_f32; // approximate; refined by on_scroll
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let visible_count =
        ((viewport_height / item_height) as usize + VISIBLE_BUFFER).min(max_display);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let scroll_items = (app.scroll_offset / item_height) as usize;
    let total_visible = visible_indices.len();
    let start = scroll_items.min(total_visible.saturating_sub(visible_count));
    let end = (start + visible_count).min(max_display).min(total_visible);

    let view_header = container(
        row![
            text(format!(
                "{} {}",
                total_visible,
                if total_visible == 1 { "match" } else { "matches" }
            ))
            .size(theme::fs(12.0))
            .style(theme::muted_text_style()),
            Space::new().width(Length::Fill),
            button(text("Cards").size(theme::fs(11.0)))
                .on_press(Message::ResultsViewChanged(ResultsView::Cards))
                .style(move |t, s| {
                    if is_table {
                        theme::ghost_button()(t, s)
                    } else {
                        theme::primary_button()(t, s)
                    }
                })
                .padding(Padding::from([3, 10])),
            button(text("Table").size(theme::fs(11.0)))
                .on_press(Message::ResultsViewChanged(ResultsView::Table))
                .style(move |t, s| {
                    if is_table {
                        theme::primary_button()(t, s)
                    } else {
                        theme::ghost_button()(t, s)
                    }
                })
                .padding(Padding::from([3, 10])),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding(Padding::from([6, 12]))
    .width(Length::Fill);

    // Batch operations toolbar — shown when >1 result is selected.
    let batch_toolbar: Option<Element<Message>> = if app.selected_paths.len() > 1 {
        Some(
            container(
                row![
                    text(format!("{} selected", app.selected_paths.len()))
                        .size(theme::fs(12.0))
                        .style(theme::muted_text_style()),
                    Space::new().width(Length::Fill),
                    button(
                        row![
                            load_icon_size("copy", 14.0),
                            text("Copy Paths").size(theme::fs(11.0)),
                        ]
                        .spacing(4)
                        .align_y(Alignment::Center),
                    )
                    .on_press(Message::CopySelectedPaths)
                    .style(theme::ghost_button())
                    .padding(Padding::from([4, 8])),
                    button(
                        row![
                            load_icon_size("external-link", 14.0),
                            text("Open All").size(theme::fs(11.0)),
                        ]
                        .spacing(4)
                        .align_y(Alignment::Center),
                    )
                    .on_press(Message::OpenSelectedFiles)
                    .style(theme::ghost_button())
                    .padding(Padding::from([4, 8])),
                    button(
                        row![
                            load_icon_size("folder-open", 14.0),
                            text("Reveal All").size(theme::fs(11.0)),
                        ]
                        .spacing(4)
                        .align_y(Alignment::Center),
                    )
                    .on_press(Message::RevealSelectedInFolder)
                    .style(theme::ghost_button())
                    .padding(Padding::from([4, 8])),
                    button(text("Clear").size(theme::fs(11.0)))
                        .on_press(Message::ClearSelection)
                        .style(theme::ghost_button())
                        .padding(Padding::from([4, 8])),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding(Padding::from([6, 12]))
            .style(theme::padded_card_container)
            .width(Length::Fill)
            .into(),
        )
    } else {
        None
    };
    // Top spacer for items above the visible range.
    let mut result_elements: Vec<Element<Message>> = Vec::new();
    if start > 0 {
        result_elements.push(
            Space::new()
                .height(Length::Fixed(start as f32 * item_height))
                .into(),
        );
    }

    for &i in visible_indices.iter().take(end).skip(start) {
        if let Some(res) = app.results.get(i) {
            if is_table {
                result_elements.push(table_row_view(
                    app.selected_index,
                    app.hovered_item_index,
                    i,
                    res,
                ));
            } else {
                result_elements.push(result_item_view(
                    app.selected_index,
                    app.hovered_item_index,
                    &app.selected_paths,
                    i,
                    res,
                ));
            }
        }
    }

    // Bottom spacer for items below the visible range.
    let remaining = total_visible.saturating_sub(end);
    if remaining > 0 {
        result_elements.push(
            Space::new()
                .height(Length::Fixed(remaining as f32 * item_height))
                .into(),
        );
    }

    if total_visible > max_display {
        result_elements.push(
            container(
                text(format!(
                    "Showing top {max_display} of {total_visible} matches. Refine query to narrow results."
                ))
                .size(theme::fs(11.0))
                .style(theme::dim_text_style()),
            )
            .padding(12)
            .center_x(Length::Fill)
            .into(),
        );
    }

    let results = scrollable(column(result_elements))
        .height(Length::Fill)
        .on_scroll(|vp| Message::ResultsScrolled(vp.absolute_offset().y));

    let mut col = column![view_header, shadcn_iced::separator::separator()];
    if is_table {
        col = col
            .push(table_header_and_filters(app))
            .push(shadcn_iced::separator::separator());
    }
    if let Some(toolbar) = batch_toolbar {
        col = col.push(toolbar);
    }
    col = col.push(results);

    container(col)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn no_results_view(_app: &App) -> Element<'_, Message> {
    container(
        column![
            load_icon_size("warning", 40.0),
            text("No matching results found")
                .size(theme::fs(17.0))
                .font(Font {
                    weight: font::Weight::Bold,
                    ..Font::default()
                }),
            text("Try adjusting your query or expanding search filters")
                .size(theme::fs(13.0))
                .style(theme::dim_text_style()),
            Space::new().height(Length::Fixed(12.0)),
            container(
                column![
                    text("Troubleshooting Suggestions:")
                        .size(theme::fs(12.0))
                        .font(Font {
                            weight: font::Weight::Bold,
                            ..Font::default()
                        }),
                    text("• Check spelling or try simpler keywords")
                        .size(theme::fs(12.0))
                        .style(theme::muted_text_style()),
                    text("• Switch between Full Text and Filename search modes")
                        .size(theme::fs(12.0))
                        .style(theme::muted_text_style()),
                    text("• Clear active file extension filters in the left sidebar")
                        .size(theme::fs(12.0))
                        .style(theme::muted_text_style()),
                ]
                .spacing(6)
            )
            .padding(16)
            .style(theme::padded_card_container)
            .max_width(500.0)
        ]
        .spacing(12)
        .align_x(Alignment::Center),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .width(Length::Fill)
    .into()
}

#[allow(clippy::too_many_lines)]
#[allow(clippy::elidable_lifetime_names)]
fn result_item_view<'a>(
    selected_index: Option<usize>,
    hovered_item_index: Option<usize>,
    selected_paths: &'a std::collections::HashSet<String>,
    i: usize,
    res: &'a FileItem,
) -> Element<'a, Message> {
    let is_selected = selected_index == Some(i);
    let is_hovered = hovered_item_index == Some(i);
    let is_batch_selected = selected_paths.contains(&res.path);

    let mut actions_row = row![].spacing(8);
    if is_hovered || is_selected {
        actions_row = actions_row.push(
            row![
                button(
                    row![
                        load_icon_size("external-link", 13.0),
                        text("Open").size(theme::fs(11.0))
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center)
                )
                .on_press(Message::OpenFile(res.path.clone()))
                .style(theme::ghost_button())
                .padding(Padding::from([4, 8])),
                button(
                    row![
                        load_icon_size("folder-open", 13.0),
                        text("Folder").size(theme::fs(11.0))
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center)
                )
                .on_press(Message::OpenFolder(res.path.clone()))
                .style(theme::ghost_button())
                .padding(Padding::from([4, 8])),
                button(load_icon_size("copy", 14.0))
                    .on_press(Message::CopyPath(res.path.clone()))
                    .style(theme::ghost_button())
                    .padding(Padding::new(5.0)),
            ]
            .spacing(4),
        );
    }

    let file_icon_name = crate::iced_ui::icons::extension_icon(res.extension.as_deref());

    let path_for_toggle = res.path.clone();
    let card_content = column![
        row![
            button(load_icon_size(
                if is_batch_selected {
                    "check-square"
                } else {
                    "square"
                },
                16.0,
            ))
            .on_press(Message::ToggleResultSelection(path_for_toggle))
            .style(theme::ghost_button())
            .padding(Padding::new(2.0)),
            load_icon_size(file_icon_name, 18.0),
            text(&*res.title).size(theme::fs(14.0)).font(Font {
                weight: font::Weight::Bold,
                ..Font::default()
            }),
            Space::new().width(Length::Fill),
            actions_row,
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        text(&res.path)
            .size(theme::fs(12.0))
            .style(theme::dim_text_style()),
        row![
            container(
                text(res.extension.as_deref().unwrap_or("FILE").to_uppercase())
                    .size(theme::fs(10.0))
                    .font(Font {
                        weight: font::Weight::Bold,
                        ..Font::default()
                    })
            )
            .padding(Padding::from([2, 6]))
            .style(|t| theme::file_badge_container(t, res.extension.as_deref())),
            container(
                text(
                    res.size
                        .map_or_else(|| "Unknown size".to_string(), crate::iced_ui::format_size)
                )
                .size(theme::fs(10.0))
            )
            .padding(Padding::from([2, 6]))
            .style(theme::badge_container),
            container(
                text(
                    res.modified
                        .map_or_else(|| "Unknown date".to_string(), crate::iced_ui::format_date)
                )
                .size(theme::fs(10.0))
            )
            .padding(Padding::from([2, 6]))
            .style(theme::badge_container),
        ]
        .spacing(6),
        if res.snippets.is_empty() {
            Element::from(Space::new().height(0))
        } else {
            let mut snippet_col = column![].spacing(6);
            for snippet in res.snippets.iter().take(3) {
                snippet_col = snippet_col.push(
                    container(parse_snippet(snippet))
                        .padding(Padding::new(8.0))
                        .width(Length::Fill)
                        .style(theme::hit_highlight_container),
                );
            }
            Element::from(snippet_col)
        }
    ]
    .spacing(8);

    let card_body = if is_selected {
        let accent_strip = container(Space::new().width(Length::Fixed(4.0)).height(Length::Fill))
            .style(|_t| container::Style {
                background: Some(iced::Background::Color(theme::ACCENT_BLUE)),
                border: iced::Border {
                    color: iced::Color::TRANSPARENT,
                    width: 0.0,
                    radius: iced::border::Radius::from(2.0),
                },
                ..Default::default()
            });

        row![
            accent_strip,
            container(card_content)
                .padding(Padding {
                    left: 10.0,
                    ..Padding::default()
                })
                .width(Length::Fill)
        ]
        .align_y(Alignment::Center)
        .width(Length::Fill)
    } else {
        row![container(card_content).width(Length::Fill)]
            .align_y(Alignment::Center)
            .width(Length::Fill)
    };

    let mut item_area = container(card_body)
        .padding(Padding::new(14.0))
        .style(if is_selected {
            theme::result_card_selected
        } else {
            theme::result_card_normal
        })
        .width(Length::Fill);

    if is_hovered && !is_selected {
        item_area = item_area.style(theme::result_card_hover);
    }

    let mouse_wrapper = mouse_area(item_area)
        .on_press(Message::ResultSelected(i))
        .on_right_press(Message::ShowContextMenu(i))
        .on_enter(Message::ItemHovered(Some(i)))
        .on_exit(Message::ItemHovered(None));

    container(mouse_wrapper)
        .padding(Padding {
            top: 3.0,
            bottom: 3.0,
            left: 10.0,
            right: 10.0,
        })
        .into()
}

fn sort_header_button(
    label: &str,
    col: Option<SortBy>,
    active: SortBy,
    descending: bool,
    width: Length,
) -> Element<'_, Message> {
    let arrow = if col == Some(active) {
        if descending { " ▼" } else { " ▲" }
    } else {
        ""
    };
    let content = text(format!("{label}{arrow}"))
        .size(theme::fs(11.5))
        .font(Font {
            weight: font::Weight::Bold,
            ..Font::default()
        });
    let btn = button(content)
        .style(theme::ghost_button())
        .padding(Padding::from([4, 6]))
        .width(width);
    match col {
        Some(sort_col) => btn.on_press(Message::SortColumnClicked(sort_col)).into(),
        None => btn.into(),
    }
}

fn table_header_and_filters(app: &App) -> Element<'_, Message> {
    let headers = row![
        sort_header_button(
            "Name",
            Some(SortBy::Name),
            app.sort_by,
            app.sort_descending,
            Length::FillPortion(3),
        ),
        sort_header_button(
            "Modified",
            Some(SortBy::DateModified),
            app.sort_by,
            app.sort_descending,
            Length::Fixed(130.0),
        ),
        sort_header_button(
            "Path",
            None,
            app.sort_by,
            app.sort_descending,
            Length::FillPortion(4),
        ),
        sort_header_button(
            "Type",
            None,
            app.sort_by,
            app.sort_descending,
            Length::Fixed(70.0),
        ),
        sort_header_button(
            "Size",
            Some(SortBy::Size),
            app.sort_by,
            app.sort_descending,
            Length::Fixed(90.0),
        ),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let filter_row = row![
        TextInput::new("eg: report", &app.column_filters.name)
            .on_input(|s| Message::ColumnFilterChanged(ColumnFilterField::Name, s))
            .size(theme::fs(11.0))
            .padding(4)
            .width(Length::FillPortion(3)),
        TextInput::new("2024-01~2024-06", &app.column_filters.modified)
            .on_input(|s| Message::ColumnFilterChanged(ColumnFilterField::Modified, s))
            .size(theme::fs(11.0))
            .padding(4)
            .width(Length::Fixed(130.0)),
        TextInput::new("eg: work", &app.column_filters.path)
            .on_input(|s| Message::ColumnFilterChanged(ColumnFilterField::Path, s))
            .size(theme::fs(11.0))
            .padding(4)
            .width(Length::FillPortion(4)),
        TextInput::new("pdf docx", &app.column_filters.ext)
            .on_input(|s| Message::ColumnFilterChanged(ColumnFilterField::Ext, s))
            .size(theme::fs(11.0))
            .padding(4)
            .width(Length::Fixed(70.0)),
        TextInput::new("1 MB~10 MB", &app.column_filters.size)
            .on_input(|s| Message::ColumnFilterChanged(ColumnFilterField::Size, s))
            .size(theme::fs(11.0))
            .padding(4)
            .width(Length::Fixed(90.0)),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    container(column![headers, filter_row].spacing(4))
        .padding(Padding::from([4, 12]))
        .style(theme::header_container)
        .width(Length::Fill)
        .into()
}

fn table_row_view(
    selected_index: Option<usize>,
    hovered_item_index: Option<usize>,
    i: usize,
    res: &FileItem,
) -> Element<'_, Message> {
    let is_selected = selected_index == Some(i);
    let is_hovered = hovered_item_index == Some(i);

    let file_name = std::path::Path::new(&res.path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&res.title);
    let folder = std::path::Path::new(&res.path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");
    let modified = res
        .modified
        .map_or_else(|| "—".to_string(), crate::iced_ui::format_date);
    let ext = res
        .extension
        .as_deref()
        .unwrap_or("FILE")
        .to_ascii_uppercase();
    let size = res
        .size
        .map_or_else(|| "—".to_string(), crate::iced_ui::format_size);

    let row_content = row![
        text(file_name)
            .size(theme::fs(12.0))
            .width(Length::FillPortion(3)),
        text(modified)
            .size(theme::fs(11.0))
            .style(theme::dim_text_style())
            .width(Length::Fixed(130.0)),
        text(folder)
            .size(theme::fs(11.0))
            .style(theme::dim_text_style())
            .width(Length::FillPortion(4)),
        text(ext)
            .size(theme::fs(10.5))
            .style(theme::muted_text_style())
            .width(Length::Fixed(70.0)),
        text(size)
            .size(theme::fs(11.0))
            .style(theme::dim_text_style())
            .width(Length::Fixed(90.0)),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let mut row_box = container(row_content)
        .padding(Padding::from([6, 12]))
        .height(Length::Fixed(32.0))
        .width(Length::Fill)
        .style(if is_selected {
            theme::result_card_selected
        } else {
            theme::result_card_normal
        });

    if is_hovered && !is_selected {
        row_box = row_box.style(theme::result_card_hover);
    }

    mouse_area(row_box)
        .on_press(Message::ResultSelected(i))
        .on_right_press(Message::ShowContextMenu(i))
        .on_enter(Message::ItemHovered(Some(i)))
        .on_exit(Message::ItemHovered(None))
        .into()
}
