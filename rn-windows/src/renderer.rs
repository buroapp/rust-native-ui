use rn_core::{Changeset, ComponentKind, PlatformRenderer, RenderError};

pub struct WindowsRenderer;

impl WindowsRenderer {
    pub fn new() -> Result<Self, RenderError> {
        Ok(Self)
    }
}

impl PlatformRenderer for WindowsRenderer {
    fn apply(&mut self, _changeset: Changeset) -> Result<(), RenderError> {
        Ok(())
    }

    fn run_event_loop(&mut self) -> Result<(), RenderError> {
        Err(RenderError::OsError(
            "windows backend not implemented in v0.1".to_string(),
        ))
    }
}

/// This function is never called but must compile. The exhaustive match over
/// `ComponentKind` means the compiler will reject any new variant that is not
/// listed here, satisfying the completeness-guard CI contract (FR-003 / C4).
#[allow(dead_code)]
fn assert_all_kinds_handled(kind: ComponentKind) {
    match kind {
        ComponentKind::Window => {}
        ComponentKind::Label => {}
        ComponentKind::Button => {}
        ComponentKind::TextInput => {}
        ComponentKind::LayoutContainer(_) => {}
        ComponentKind::Checkbox => {}
        ComponentKind::Slider => {}
        ComponentKind::ProgressBar => {}
        ComponentKind::Separator => {}
        ComponentKind::TextArea => {}
        ComponentKind::Dropdown => {}
        ComponentKind::Image => {}
        ComponentKind::Stepper => {}
        ComponentKind::ColorWell => {}
        ComponentKind::DatePicker => {}
        ComponentKind::RadioGroup => {}
        ComponentKind::ScrollView => {}
        ComponentKind::FlatList => {}
        ComponentKind::TouchableOpacity => {}
        ComponentKind::SafeAreaView => {}
        ComponentKind::ActivityIndicator => {}
        ComponentKind::Modal => {}
        ComponentKind::Popover => {}
        ComponentKind::RichText => {}
        ComponentKind::Card => {}
        ComponentKind::Badge => {}
        ComponentKind::Skeleton => {}
        ComponentKind::SegmentedControl => {}
        ComponentKind::SearchBar => {}
        ComponentKind::Sidebar => {}
        ComponentKind::SidebarItem => {}
        ComponentKind::SidebarSection => {}
        ComponentKind::GroupedSection => {}
        ComponentKind::SplitView => {}
        ComponentKind::Toolbar => {}
        ComponentKind::Switch => {}
        ComponentKind::StatusBar => {}
        ComponentKind::ContentArea => {}
        ComponentKind::ListTile => {}
        ComponentKind::Scaffold => {}
        ComponentKind::TabView => {}
        ComponentKind::TabItem => {}
        ComponentKind::TitleBar => {}
        ComponentKind::IconButton => {}
        ComponentKind::DisclosureButton => {}
        ComponentKind::HelpButton => {}
        ComponentKind::PopupButton => {}
        ComponentKind::BackButton => {}
        ComponentKind::CapacityIndicator => {}
        ComponentKind::RatingIndicator => {}
        ComponentKind::RelevanceIndicator => {}
        ComponentKind::Tooltip => {}
        ComponentKind::TimePicker => {}
        ComponentKind::GraphicalTimePicker => {}
        ComponentKind::ToolbarIconButton => {}
        ComponentKind::ToolbarSpacer => {}
        ComponentKind::ToolbarDivider => {}
        ComponentKind::ToolbarPopup => {}
        ComponentKind::AlertDialog => {}
        ComponentKind::Sheet => {}
    }
}
