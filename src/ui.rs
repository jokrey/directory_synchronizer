use std::collections::HashMap;
use alignment::{Alignment, Vertical};
use iced::{Element, Application, Settings, Theme, executor, Command, Length, alignment, Font};
use iced::widget::{button, checkbox, column, Column, container, row, scrollable, Space, text};
use iced::widget::scrollable::Properties;
use crate::differences::{apply_diffs_source_to_target_with_prints, Difference, find_differences, verify_source_fully_newer_than_target};

pub(crate) fn start_synchronization_ui(source_path: String, target_path: String) -> iced::Result {
    SynchronizerUI::run(Settings::with_flags( SynchronizerUiFlags { source_path,
        target_path,
    }))
}


struct SynchronizerUI {
    source_path: String,
    target_path: String,
    selected_differences: Vec<(Difference, bool)>,
    problems: HashMap<Difference, String>
}

impl SynchronizerUI {
    pub(crate) fn re_run_analysis(&mut self) {
        let newly_found = find_differences(&self.source_path, &self.target_path);
        self.selected_differences.clear();
        self.problems = verify_source_fully_newer_than_target(&newly_found);
        for d in newly_found {
            let has_problem = self.problems.get(&d).is_some();
            self.selected_differences.push((d, !has_problem));
        }
    }
    pub(crate) fn apply_selected_changes(&mut self) {
        apply_diffs_source_to_target_with_prints(
            &self.source_path, &self.target_path,
            self.selected_differences.iter().filter(|(_, selected)| *selected).map(|(d, _)| d)
        );
        self.re_run_analysis();
    }
}

struct SynchronizerUiFlags {
    source_path: String,
    target_path: String
}

#[derive(Debug, Clone)]
enum SynchronizerUiMessage {
    CHECKBOX(bool, usize),
    AnalyzeDirectories,
    ApplySelectedChanges
}

impl Application for SynchronizerUI {
    type Executor = executor::Default;
    type Message = SynchronizerUiMessage;
    type Theme = Theme;
    type Flags = SynchronizerUiFlags;

    fn new(flags: SynchronizerUiFlags) -> (SynchronizerUI, Command<Self::Message>) {
        let r = (
            SynchronizerUI { source_path: flags.source_path, target_path: flags.target_path, selected_differences: Vec::new(), problems: HashMap::new() },
            Command::none(),
        );
        // r.0.re_run_analysis(); //blocks ui for too long
        return r
    }

    fn title(&self) -> String { return String::from("Directory Synchronizer UI"); }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            // React to messages
            SynchronizerUiMessage::CHECKBOX(v, i) => {
                self.selected_differences[i].1 = v;
            },
            SynchronizerUiMessage::AnalyzeDirectories => {
                self.re_run_analysis();
            },
            SynchronizerUiMessage::ApplySelectedChanges => {
                self.apply_selected_changes();
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        let header = row![
            text(&self.source_path).font(Font::with_name("Monospaced")),
            text(" --> ").font(Font::with_name("Monospaced")),
            text(&self.target_path).font(Font::with_name("Monospaced")),
        ].spacing(22);

        let analyze = button("Analyze Directories").on_press(SynchronizerUiMessage::AnalyzeDirectories);

        let apply = button("Apply selected changes").on_press(SynchronizerUiMessage::ApplySelectedChanges);

        let results = if self.selected_differences.is_empty() {
            Element::from(text("No differences found."))
        } else {
            let mut children = Vec::with_capacity(self.selected_differences.len());
            for (i, (d, active)) in self.selected_differences.iter().enumerate() {
                let checkbox =
                    checkbox("", *active, move |b| SynchronizerUiMessage::CHECKBOX(b, i));
                match self.problems.get(d) {
                    None => {
                        children.push(Element::from(
                            row![
                                checkbox,
                                column![
                                    text(d.describe_short()),
                                    text(format!("    in directory: \"{}\"", d.get_directory_path(self.source_path.len(), self.target_path.len()))),
                                ]
                            ].align_items(Alignment::Center)
                        ));
                    },
                    Some(desc) => {
                        children.push(Element::from(
                            row![
                                checkbox,
                                column![
                                    text(d.describe_short()),
                                    text(format!("    in directory: \"{}\"", d.get_directory_path(self.source_path.len(), self.target_path.len()))),
                                    text(format!("    Problem: {desc}"))
                                ]
                            ].align_items(Alignment::Center)
                        ));
                    }
                }
                children.push(Element::from(Space::with_height(3)));
            }
            Element::from(scrollable(
                Column::with_children(children)
            )
            .width(Length::Fill)
            .height(Length::FillPortion(2))
            .direction(scrollable::Direction::Vertical(
                Properties::new()
            )))
        };

        container(column![
            container(header).width(Length::Fill).height(Length::Shrink).center_x().align_y(Vertical::Top),
            container(analyze).width(Length::Fill).center_x(),
            container(apply).width(Length::Fill).center_x(),
            container(results).width(Length::Fill).height(Length::Shrink).center_x(),
        ]).width(Length::Fill).height(Length::Fill).center_x().into()
    }
}