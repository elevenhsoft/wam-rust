use std::path::PathBuf;

use iced::{
    alignment::{Horizontal, Vertical},
    theme::{self, Custom, Palette},
    widget::{
        button::{self},
        column, image, pick_list, row, scrollable, svg, text, text_input, toggler, Button,
        Container, Row, TextInput,
    },
    Alignment, Application, BorderRadius, Color, Command, Length, Theme,
};
use iced_aw::{modal, Card, Wrap};
use url::Url;
use xdg::BaseDirectories;

use crate::common::{
    find_icons, get_icon_name_from_url, get_supported_browsers, get_webapps, image_from_memory,
    move_icon, svg_from_memory, Browser, WebAppLauncher,
};

#[derive(Debug, Clone)]
pub enum Buttons {
    SearchFavicon,
    Favicon(String),
    Edit(Box<WebAppLauncher>),
    Delete(Box<WebAppLauncher>),
    Navbar(bool),
    IsolatedProfile(bool),
    Incognito(bool),
}

#[derive(Debug, Clone)]
pub enum AppMessage {
    PushIcon(Icon),
    FoundIcons(Vec<String>),
    SetIcon(Icon),
    // modal
    OpenModal,
    CloseModal,
    CancelButtonPressed,
    PerformIconSearch,
    CustomIconsSearch(String),
    // common
    Result,
    Clicked(Buttons),
    Title(String),
    Url(String),
    Arguments(String),
    Browser(Browser),
    Category(String),
    ErrorLoadingIcon,
    SelectIcon(Icon),
}

#[derive(Debug, Clone)]
pub enum IconType {
    Raster(image::Handle),
    Svg(svg::Handle),
}

#[derive(Debug, Clone)]
pub struct Icon {
    pub icon: IconType,
    pub path: String,
}

impl Icon {
    pub fn new(icon: IconType, path: String) -> Self {
        Self { icon, path }
    }
}

pub struct Wam {
    pub icons_paths: Vec<String>,
    pub icons: Option<Vec<Icon>>,
    pub app_codename: Option<String>,
    pub app_title: String,
    pub app_url: String,
    pub app_icon: String,
    pub app_parameters: String,
    pub app_category: String,
    pub app_browser_name: String,
    pub app_browser: Browser,
    pub app_navbar: bool,
    pub app_incognito: bool,
    pub app_isolated: bool,
    show_modal: bool,
    icon_searching: String,
    selected_icon: Option<Icon>,
    app_browsers: Vec<Browser>,
    edit_mode: bool,
    launcher: Option<Box<WebAppLauncher>>,
    app_base_dir: PathBuf,
}

impl Application for Wam {
    type Executor = iced::executor::Default;

    type Message = AppMessage;

    type Theme = iced::Theme;

    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let browsers = get_supported_browsers();
        let browser = &browsers[0];

        let base_dir = BaseDirectories::new().expect("cant follow base directories");
        let local_share = base_dir.get_data_home();
        let wam_rust_path = local_share.join("wam-rust");
        (
            Wam {
                icons_paths: Vec::new(),
                icons: Some(Vec::new()),
                app_codename: None,
                app_title: String::new(),
                app_url: String::new(),
                app_icon: String::new(),
                app_parameters: String::new(),
                app_category: String::from("Web"),
                app_browser_name: String::from("Browser"),
                app_browser: browser.clone(),
                app_navbar: false,
                app_incognito: false,
                app_isolated: true,
                show_modal: false,
                icon_searching: String::new(),
                selected_icon: None,
                app_browsers: browsers,
                edit_mode: false,
                launcher: None,
                app_base_dir: wam_rust_path,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("WAM Rust - Web App Manager")
    }

    fn theme(&self) -> Self::Theme {
        let gsettings = "gsettings";

        let cs_cmd = std::process::Command::new(gsettings)
            .arg("get")
            .arg("org.gnome.desktop.interface")
            .arg("color-scheme")
            .output();

        let mut palette: Palette = Palette {
            background: Color::WHITE,
            text: Color::from_rgba(0.0, 0.0, 0.0, 0.75),
            primary: Color::from_rgb(0.0, 0.28, 0.73),
            success: Color::from_rgb(0.24, 0.57, 0.25),
            danger: Color::from_rgb(0.90, 0.17, 0.31),
        };

        if let Ok(cmd) = cs_cmd {
            let color_scheme = String::from_utf8_lossy(&cmd.stdout);

            if color_scheme.trim().contains("dark") {
                palette = Palette {
                    background: Color::from_rgb(0.1, 0.1, 0.1),
                    text: Color::from_rgba(1.0, 1.0, 1.0, 0.75),
                    primary: Color::from_rgb(0.0, 0.28, 0.73),
                    success: Color::from_rgb(0.24, 0.57, 0.25),
                    danger: Color::from_rgb(0.90, 0.17, 0.31),
                }
            }
        };

        iced::Theme::Custom(Box::new(Custom::new(palette)))
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            AppMessage::Title(text) => {
                self.app_title = text;

                Command::none()
            }
            AppMessage::Url(url) => {
                self.app_url = url;

                Command::none()
            }
            AppMessage::Arguments(args) => {
                self.app_parameters = args;

                Command::none()
            }
            AppMessage::PushIcon(icon) => {
                if let Some(vec) = self.icons.as_mut() {
                    if vec.is_empty() {
                        self.selected_icon = Some(icon.clone());
                        if !&icon.path.starts_with("http") {
                            self.app_icon = icon.path.clone()
                        } else {
                            self.app_icon =
                                move_icon(icon.path.clone(), self.app_title.replace(' ', ""))
                                    .expect("cant download icon")
                        }
                    }

                    vec.push(icon.clone());
                }
                Command::none()
            }
            AppMessage::FoundIcons(result) => {
                if !result.is_empty() {
                    let commands: Vec<Command<AppMessage>> = result
                        .into_iter()
                        .map(|path| {
                            let is_svg = path.ends_with(".svg");

                            match is_svg {
                                true => {
                                    Command::perform(svg_from_memory(path), |result| match result {
                                        Ok(icon) => AppMessage::PushIcon(icon),
                                        Err(_) => AppMessage::ErrorLoadingIcon,
                                    })
                                }
                                false => {
                                    Command::perform(image_from_memory(path), |result| match result
                                    {
                                        Ok(icon) => AppMessage::PushIcon(icon),
                                        Err(_) => AppMessage::ErrorLoadingIcon,
                                    })
                                }
                            }
                        })
                        .collect();

                    Command::batch(commands)
                } else {
                    Command::none()
                }
            }
            AppMessage::Clicked(btn) => match btn {
                Buttons::SearchFavicon => {
                    if let Some(icons) = self.icons.as_mut() {
                        icons.clear()
                    };

                    if !self.app_url.is_empty() {
                        let url = self.app_url.clone();

                        let to_find = if url.starts_with("http://") || url.starts_with("https://") {
                            get_icon_name_from_url(url.clone())
                        } else {
                            let prefix = "https://";
                            self.app_url = format!("{}{}", prefix, url);

                            get_icon_name_from_url(self.app_url.clone())
                        };

                        Command::perform(find_icons(to_find, Some(self.app_url.clone())), |icons| {
                            AppMessage::FoundIcons(icons)
                        })
                    } else {
                        Command::none()
                    }
                }
                Buttons::Favicon(path) => {
                    let is_svg = path.ends_with(".svg");

                    match is_svg {
                        true => Command::perform(svg_from_memory(path), |result| match result {
                            Ok(icon) => AppMessage::SetIcon(icon),
                            Err(_) => AppMessage::ErrorLoadingIcon,
                        }),
                        false => Command::perform(image_from_memory(path), |result| match result {
                            Ok(icon) => AppMessage::SetIcon(icon),
                            Err(_) => AppMessage::ErrorLoadingIcon,
                        }),
                    }
                }
                Buttons::Edit(launcher) => {
                    self.edit_mode = true;
                    self.launcher = Some(launcher.clone());

                    self.app_title = launcher.name;
                    self.app_url = launcher.url;
                    self.app_icon = launcher.icon.clone();
                    self.app_parameters = launcher.custom_parameters;
                    self.app_category = launcher.category;
                    self.app_browser =
                        Browser::web_browser(launcher.web_browser.name).expect("browser not found");
                    self.app_navbar = launcher.navbar;
                    self.app_incognito = launcher.is_incognito;

                    let is_svg = launcher.icon.ends_with(".svg");

                    match is_svg {
                        true => {
                            Command::perform(
                                svg_from_memory(launcher.icon),
                                |result| match result {
                                    Ok(icon) => AppMessage::SetIcon(icon),
                                    Err(_) => AppMessage::ErrorLoadingIcon,
                                },
                            )
                        }
                        false => {
                            Command::perform(
                                image_from_memory(launcher.icon),
                                |result| match result {
                                    Ok(icon) => AppMessage::SetIcon(icon),
                                    Err(_) => AppMessage::ErrorLoadingIcon,
                                },
                            )
                        }
                    }
                }
                Buttons::Delete(launcher) => {
                    let _ = launcher.delete();

                    Command::none()
                }
                Buttons::Navbar(selected) => {
                    self.app_navbar = selected;

                    Command::none()
                }
                Buttons::Incognito(selected) => {
                    self.app_incognito = selected;

                    Command::none()
                }
                Buttons::IsolatedProfile(selected) => {
                    self.app_isolated = selected;

                    Command::none()
                }
            },
            AppMessage::Browser(browser) => {
                self.app_browser = browser;

                Command::none()
            }
            AppMessage::Result => {
                let launcher = if let Some(launcher) = self.launcher.to_owned() {
                    let _ = launcher.delete();
                    Box::new(WebAppLauncher::new(
                        self.app_title.clone(),
                        Some(launcher.codename),
                        self.app_url.clone(),
                        self.app_icon.clone(),
                        self.app_category.clone(),
                        self.app_browser.clone(),
                        self.app_parameters.clone(),
                        self.app_isolated,
                        self.app_navbar,
                        self.app_incognito,
                    ))
                } else {
                    Box::new(WebAppLauncher::new(
                        self.app_title.clone(),
                        None,
                        self.app_url.clone(),
                        self.app_icon.clone(),
                        self.app_category.clone(),
                        self.app_browser.clone(),
                        self.app_parameters.clone(),
                        self.app_isolated,
                        self.app_navbar,
                        self.app_incognito,
                    ))
                };

                if launcher.is_valid {
                    let _ = launcher.create();
                }

                Command::none()
            }
            AppMessage::Category(category) => {
                self.app_category = category;

                Command::none()
            }
            AppMessage::OpenModal => {
                self.show_modal = true;

                Command::none()
            }
            AppMessage::CloseModal => {
                self.show_modal = false;

                Command::none()
            }
            AppMessage::CancelButtonPressed => {
                self.show_modal = false;

                Command::none()
            }
            AppMessage::ErrorLoadingIcon => Command::none(),
            AppMessage::SetIcon(icon) => {
                self.show_modal = false;

                let path = icon.path;

                if let Ok(saved) = move_icon(path, self.app_title.clone()) {
                    self.app_icon = saved.clone();

                    if saved.ends_with(".svg") {
                        Command::perform(svg_from_memory(saved), |result| {
                            AppMessage::SelectIcon(result.unwrap())
                        })
                    } else {
                        Command::perform(image_from_memory(saved), |result| {
                            AppMessage::SelectIcon(result.unwrap())
                        })
                    }
                } else {
                    Command::none()
                }
            }
            AppMessage::SelectIcon(ico) => {
                self.selected_icon = Some(ico);

                Command::none()
            }
            AppMessage::CustomIconsSearch(field) => {
                self.icon_searching = field;

                Command::none()
            }
            AppMessage::PerformIconSearch => {
                if let Some(icons) = self.icons.as_mut() {
                    icons.clear()
                };

                if !self.icon_searching.is_empty() {
                    Command::perform(
                        find_icons(self.icon_searching.clone(), Some(self.app_url.clone())),
                        AppMessage::FoundIcons,
                    )
                } else {
                    Command::none()
                }
            }
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
        let app_title = text_input("Title", &self.app_title)
            .on_input(AppMessage::Title)
            .padding(10)
            .width(Length::Fixed(340.))
            .style(theme::TextInput::Custom(Box::new(InputField)));
        let app_url = text_input("URL", &self.app_url)
            .on_input(AppMessage::Url)
            .padding(10)
            .width(Length::Fixed(340.))
            .style(theme::TextInput::Custom(Box::new(InputField)));

        let col = column![app_title, app_url].spacing(14);

        let search_ico = &self.app_base_dir.join("icons/search.svg");
        let search_ico = search_ico.to_str().expect("cant find needed search icon");

        let dl_btn = Button::new(
            svg(svg::Handle::from_path(search_ico))
                .style(theme::Svg::Custom(Box::new(AdaptiveSvg)))
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(AppMessage::Clicked(Buttons::SearchFavicon))
        .width(Length::Fixed(96.))
        .height(Length::Fixed(96.))
        .style(theme::Button::Custom(Box::new(CustomButton)));

        let icons = self.icons.clone().unwrap();

        let icon = if !icons.is_empty() || !self.app_icon.is_empty() {
            self.icon_picker_icon(self.selected_icon.clone())
        } else {
            self.icon_picker_icon(None)
        };
        let row = row![col, dl_btn, icon].spacing(12).width(Length::Fill);

        let app_arguments = text_input("Non-standard arguments", &self.app_parameters)
            .on_input(AppMessage::Arguments)
            .padding(10)
            .width(Length::Fill)
            .style(theme::TextInput::Custom(Box::new(InputField)));

        let categories = [
            String::from("Web"),
            String::from("Accesories"),
            String::from("Education"),
            String::from("Games"),
            String::from("Graphics"),
            String::from("Internet"),
            String::from("Office"),
            String::from("Programming"),
            String::from("Sound & Video"),
        ];

        let category = pick_list(
            categories.to_vec(),
            Some(self.app_category.clone()),
            AppMessage::Category,
        )
        .width(Length::Fill)
        .padding(10);

        let browser_specific = match self.app_browser._type {
            crate::common::BrowserType::Firefox => {
                toggler(String::from("Nav Bar"), self.app_navbar, |b| {
                    AppMessage::Clicked(Buttons::Navbar(b))
                })
                .width(Length::Fill)
            }
            crate::common::BrowserType::FirefoxFlatpak => {
                toggler(String::from("Nav Bar"), self.app_navbar, |b| {
                    AppMessage::Clicked(Buttons::Navbar(b))
                })
                .width(Length::Fill)
            }
            crate::common::BrowserType::Librewolf => {
                toggler(String::from("Nav Bar"), self.app_navbar, |b| {
                    AppMessage::Clicked(Buttons::Navbar(b))
                })
                .width(Length::Fill)
            }
            crate::common::BrowserType::WaterfoxFlatpak => {
                toggler(String::from("Nav Bar"), self.app_navbar, |b| {
                    AppMessage::Clicked(Buttons::Navbar(b))
                })
                .width(Length::Fill)
            }
            _ => toggler(String::from("Isolated Profile"), self.app_isolated, |b| {
                AppMessage::Clicked(Buttons::IsolatedProfile(b))
            })
            .width(Length::Fill),
        };

        let incognito = toggler(String::from("Private Mode"), self.app_incognito, |b| {
            AppMessage::Clicked(Buttons::Incognito(b))
        })
        .width(Length::Fill);

        let cat_row = row![category, incognito, browser_specific]
            .align_items(Alignment::Center)
            .spacing(20);

        let app_browsers = pick_list(
            self.app_browsers.clone(),
            Some(self.app_browser.clone()),
            AppMessage::Browser,
        )
        .width(Length::Fill)
        .padding(10);

        let app_done = Button::new("Done")
            .on_press(AppMessage::Result)
            .width(Length::Fill)
            .padding(10);

        let browsers_row = row![app_browsers, app_done].spacing(20);

        let mut app_list = column!().spacing(10);
        let webapps = get_webapps();

        for app in webapps.iter() {
            match app {
                Ok(data) => {
                    let edit = Button::new("Edit")
                        .on_press(AppMessage::Clicked(Buttons::Edit(Box::new(data.clone()))))
                        .width(Length::Fixed(90.))
                        .style(theme::Button::Primary);
                    let delete = Button::new("Delete")
                        .on_press(AppMessage::Clicked(Buttons::Delete(Box::new(data.clone()))))
                        .width(Length::Fixed(90.))
                        .style(theme::Button::Destructive);

                    let host = Url::parse(&data.url).expect("cant parse url");
                    let host = host.host().unwrap();

                    let name = Button::new(text(data.name.clone()))
                        .width(Length::FillPortion(2))
                        .style(theme::Button::Positive);
                    let url = Button::new(text(host))
                        .width(Length::FillPortion(3))
                        .style(theme::Button::Positive);

                    let row = row![edit, delete, name, url]
                        .spacing(10)
                        .align_items(Alignment::Center);
                    app_list = app_list.push(row);
                }
                Err(e) => tracing::error!("Error reading web app: {}", e),
            }
        }

        let mut installed = column![];

        if !webapps.is_empty() {
            installed = installed.push(text("INSTALLED").size(26.)).spacing(10);

            let scrollable_list = scrollable(app_list).width(Length::Fill);

            installed = installed.push(scrollable_list);
        }

        let col = column![row, app_arguments, cat_row, browsers_row].spacing(20);
        let col = column![col, installed].spacing(50);

        let underlay = Container::new(col).padding(30);

        let overlay = if self.show_modal {
            Some(
                Card::new(
                    text("Icon Picker"),
                    self.icons_container(self.icons.clone()),
                )
                .foot(
                    Row::new().spacing(10).padding(5).width(Length::Fill).push(
                        Button::new(text("Cancel").horizontal_alignment(Horizontal::Center))
                            .width(Length::Fill)
                            .on_press(AppMessage::CancelButtonPressed),
                    ),
                )
                .max_width(500.0)
                .max_height(600.0)
                .height(Length::Shrink)
                .on_close(AppMessage::CloseModal),
            )
        } else {
            None
        };

        modal(underlay, overlay)
            .backdrop(AppMessage::CloseModal)
            .on_esc(AppMessage::CloseModal)
            .align_y(Vertical::Center)
            .into()
    }
}

impl Wam {
    fn icons_container(&self, icons: Option<Vec<Icon>>) -> iced::Element<'static, AppMessage> {
        let search_field = TextInput::new("Search for icon", &self.icon_searching)
            .on_input(AppMessage::CustomIconsSearch)
            .on_submit(AppMessage::PerformIconSearch)
            .padding(10)
            .width(Length::Fill);

        let mut container = Wrap::new().max_width(500.);

        if icons.is_some() {
            for ico in icons.unwrap().iter() {
                let btn = match ico.clone().icon {
                    IconType::Raster(icon) => Button::new(image(icon))
                        .width(Length::Fixed(96.))
                        .height(Length::Fixed(96.))
                        .on_press(AppMessage::Clicked(Buttons::Favicon(ico.path.clone())))
                        .style(theme::Button::Custom(Box::new(CustomButton))),
                    IconType::Svg(icon) => Button::new(svg(icon))
                        .width(Length::Fixed(96.))
                        .height(Length::Fixed(96.))
                        .on_press(AppMessage::Clicked(Buttons::Favicon(ico.path.clone())))
                        .style(theme::Button::Custom(Box::new(CustomButton))),
                };
                container = container.push(btn);
            }
        }

        let col = column![search_field, container].spacing(20);

        scrollable(col).into()
    }

    fn icon_picker_icon(&self, icon: Option<Icon>) -> iced::Element<'static, AppMessage> {
        let ico = if let Some(ico) = icon {
            match ico.icon {
                IconType::Raster(data) => {
                    Button::new(image(data).width(Length::Fill).height(Length::Fill))
                        .on_press(AppMessage::OpenModal)
                        .width(Length::Fixed(96.))
                        .height(Length::Fixed(96.))
                        .style(theme::Button::Custom(Box::new(CustomButton)))
                }
                IconType::Svg(data) => {
                    Button::new(svg(data).width(Length::Fill).height(Length::Fill))
                        .on_press(AppMessage::OpenModal)
                        .width(Length::Fixed(96.))
                        .height(Length::Fixed(96.))
                        .style(theme::Button::Custom(Box::new(CustomButton)))
                }
            }
        } else {
            let default_ico = &self.app_base_dir.join("icons/moleskine-icon.svg");
            let default_ico = default_ico.to_str().expect("cant find needed icon");
            let default_icon_path = String::from(default_ico);
            let handler = svg::Handle::from_path(default_icon_path);
            let default = svg(handler).style(theme::Svg::Custom(Box::new(AdaptiveSvg)));

            Button::new(default)
                .width(Length::Fill)
                .height(Length::Fill)
                .on_press(AppMessage::OpenModal)
                .width(Length::Fixed(96.))
                .height(Length::Fixed(96.))
                .style(theme::Button::Custom(Box::new(CustomButton)))
        };

        Container::new(ico).into()
    }
}

struct CustomButton;

impl button::StyleSheet for CustomButton {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(iced::Background::Color(Color::TRANSPARENT)),
            ..Default::default()
        }
    }

    fn disabled(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);

        button::Appearance {
            shadow_offset: iced::Vector::default(),
            background: active.background.map(|background| match background {
                iced::Background::Color(color) => iced::Background::Color(iced::Color {
                    a: color.a * 0.5,
                    ..color
                }),
                iced::Background::Gradient(gradient) => {
                    iced::Background::Gradient(gradient.mul_alpha(0.5))
                }
            }),
            text_color: iced::Color {
                a: active.text_color.a * 0.5,
                ..active.text_color
            },
            ..active
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);

        button::Appearance { ..active }
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            shadow_offset: iced::Vector::default(),
            ..self.active(style)
        }
    }
}

struct InputField;

impl text_input::StyleSheet for InputField {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: iced::Background::Color(Color::TRANSPARENT),
            border_radius: BorderRadius::from(4.),
            border_width: 1.,
            border_color: style.palette().text,
            icon_color: style.palette().text,
        }
    }

    fn focused(&self, style: &Self::Style) -> text_input::Appearance {
        let active = self.active(style);
        text_input::Appearance {
            border_width: 1.,
            border_color: Color::from_rgba(0.76, 0.76, 0.76, 0.20),
            ..active
        }
    }

    fn placeholder_color(&self, style: &Self::Style) -> Color {
        style.palette().text
    }

    fn value_color(&self, style: &Self::Style) -> Color {
        style.palette().text
    }

    fn disabled_color(&self, style: &Self::Style) -> Color {
        style.palette().text
    }

    fn selection_color(&self, _style: &Self::Style) -> Color {
        Color::from_rgb(128., 191., 255.)
    }

    fn disabled(&self, style: &Self::Style) -> text_input::Appearance {
        let active = self.active(style);

        text_input::Appearance {
            background: iced::Background::Color(Color::TRANSPARENT),
            border_width: 0.,
            border_color: Color::TRANSPARENT,
            ..active
        }
    }
}

struct AdaptiveSvg;

impl svg::StyleSheet for AdaptiveSvg {
    type Style = Theme;

    fn appearance(&self, style: &Self::Style) -> svg::Appearance {
        svg::Appearance {
            color: Some(style.palette().text),
        }
    }
}
