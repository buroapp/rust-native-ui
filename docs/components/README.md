# rn component catalog

60 components, native AppKit controls on macOS. Each screenshot is the live component rendered by the `storybook` example.

Regenerate with `scripts/gen-docs.sh`.

## Basic

| Component | What it is |
|---|---|
| [Label](#label) | Static text |
| [Button](#button) | Push button |
| [TextInput](#textinput) | Single-line text entry |
| [TextArea](#textarea) | Multi-line text entry |
| [Checkbox](#checkbox) | Binary choice with a label |
| [Separator](#separator) | Rule between regions |
| [Image](#image) | Icon or picture |
| [Badge](#badge) | Small status label |
| [Skeleton](#skeleton) | Loading placeholder |

## Range

| Component | What it is |
|---|---|
| [Slider](#slider) | Continuous value in a range |
| [ProgressBar](#progressbar) | Determinate progress |
| [Stepper](#stepper) | Discrete numeric stepper |
| [ActivityIndicator](#activityindicator) | Indeterminate progress |
| [CapacityIndicator](#capacityindicator) | Segmented capacity meter |
| [RatingIndicator](#ratingindicator) | Star rating |
| [RelevanceIndicator](#relevanceindicator) | Search relevance meter |

## Choice

| Component | What it is |
|---|---|
| [Dropdown](#dropdown) | List of options |
| [PopupButton](#popupbutton) | Bezeled pull-down |
| [RadioGroup](#radiogroup) | Exclusive choices |
| [SegmentedControl](#segmentedcontrol) | Horizontal exclusive choice |
| [Switch](#switch) | On/off toggle |
| [ColorWell](#colorwell) | Colour picker swatch |

## Pickers

| Component | What it is |
|---|---|
| [DatePicker](#datepicker) | Calendar date |
| [TimePicker](#timepicker) | Time of day |
| [GraphicalTimePicker](#graphicaltimepicker) | Clock-face time picker |

## Lists

| Component | What it is |
|---|---|
| [Sidebar](#sidebar) | Navigation rail |
| [SidebarItem](#sidebaritem) | A sidebar row |
| [SidebarSection](#sidebarsection) | Group header in a sidebar |
| [ListTile](#listtile) | Settings-style row |
| [GroupedSection](#groupedsection) | Grouped rows with dividers |
| [FlatList](#flatlist) | Virtualised list |
| [ScrollView](#scrollview) | Scrollable region |
| [TouchableOpacity](#touchableopacity) | Pressing container |

## Layout

| Component | What it is |
|---|---|
| [LayoutContainer](#layoutcontainer) | Stacking container |
| [Card](#card) | Grouped content surface |
| [ContentArea](#contentarea) | Main window pane |
| [Scaffold](#scaffold) | Window structure |
| [SplitView](#splitview) | Resizable panes |
| [TabView](#tabview) | Native tabbed panes |
| [TabItem](#tabitem) | One page of a TabView |
| [SafeAreaView](#safeareaview) | Inset container |
| [SearchBar](#searchbar) | Search field |

## Chrome

| Component | What it is |
|---|---|
| [Window](#window) | Root window |
| [TitleBar](#titlebar) | Window title strip |
| [Toolbar](#toolbar) | Action bar |
| [ToolbarIconButton](#toolbariconbutton) | Toolbar icon action |
| [ToolbarSpacer](#toolbarspacer) | Flexible gap |
| [ToolbarDivider](#toolbardivider) | Rule between actions |
| [ToolbarPopup](#toolbarpopup) | Toolbar pull-down |
| [StatusBar](#statusbar) | Bottom status strip |

## Buttons

| Component | What it is |
|---|---|
| [IconButton](#iconbutton) | Icon-only button |
| [BackButton](#backbutton) | Navigation chevron |
| [DisclosureButton](#disclosurebutton) | Show/hide toggle |
| [HelpButton](#helpbutton) | Contextual help |

## Text

| Component | What it is |
|---|---|
| [RichText](#richtext) | Mixed formatting |
| [Tooltip](#tooltip) | Hint on a control |

## Dialogs

| Component | What it is |
|---|---|
| [Modal](#modal) | Blocking overlay |
| [Popover](#popover) | Anchored overlay |
| [AlertDialog](#alertdialog) | Alert with choice |
| [Sheet](#sheet) | Window-attached modal |

---

### Label

*Basic* — Static text

![Label](img/Label.png)

Open live: `cargo run -p storybook --features macos -- --select Label`

### Button

*Basic* — Push button

![Button](img/Button.png)

Open live: `cargo run -p storybook --features macos -- --select Button`

### TextInput

*Basic* — Single-line text entry

![TextInput](img/TextInput.png)

Open live: `cargo run -p storybook --features macos -- --select TextInput`

### TextArea

*Basic* — Multi-line text entry

![TextArea](img/TextArea.png)

Open live: `cargo run -p storybook --features macos -- --select TextArea`

### Checkbox

*Basic* — Binary choice with a label

![Checkbox](img/Checkbox.png)

Open live: `cargo run -p storybook --features macos -- --select Checkbox`

### Separator

*Basic* — Rule between regions

![Separator](img/Separator.png)

Open live: `cargo run -p storybook --features macos -- --select Separator`

### Image

*Basic* — Icon or picture

![Image](img/Image.png)

Open live: `cargo run -p storybook --features macos -- --select Image`

### Badge

*Basic* — Small status label

![Badge](img/Badge.png)

Open live: `cargo run -p storybook --features macos -- --select Badge`

### Skeleton

*Basic* — Loading placeholder

![Skeleton](img/Skeleton.png)

Open live: `cargo run -p storybook --features macos -- --select Skeleton`

### Slider

*Range* — Continuous value in a range

![Slider](img/Slider.png)

Open live: `cargo run -p storybook --features macos -- --select Slider`

### ProgressBar

*Range* — Determinate progress

![ProgressBar](img/ProgressBar.png)

Open live: `cargo run -p storybook --features macos -- --select ProgressBar`

### Stepper

*Range* — Discrete numeric stepper

![Stepper](img/Stepper.png)

Open live: `cargo run -p storybook --features macos -- --select Stepper`

### ActivityIndicator

*Range* — Indeterminate progress

![ActivityIndicator](img/ActivityIndicator.png)

Open live: `cargo run -p storybook --features macos -- --select ActivityIndicator`

### CapacityIndicator

*Range* — Segmented capacity meter

![CapacityIndicator](img/CapacityIndicator.png)

Open live: `cargo run -p storybook --features macos -- --select CapacityIndicator`

### RatingIndicator

*Range* — Star rating

![RatingIndicator](img/RatingIndicator.png)

Open live: `cargo run -p storybook --features macos -- --select RatingIndicator`

### RelevanceIndicator

*Range* — Search relevance meter

![RelevanceIndicator](img/RelevanceIndicator.png)

Open live: `cargo run -p storybook --features macos -- --select RelevanceIndicator`

### Dropdown

*Choice* — List of options

![Dropdown](img/Dropdown.png)

Open live: `cargo run -p storybook --features macos -- --select Dropdown`

### PopupButton

*Choice* — Bezeled pull-down

![PopupButton](img/PopupButton.png)

Open live: `cargo run -p storybook --features macos -- --select PopupButton`

### RadioGroup

*Choice* — Exclusive choices

![RadioGroup](img/RadioGroup.png)

Open live: `cargo run -p storybook --features macos -- --select RadioGroup`

### SegmentedControl

*Choice* — Horizontal exclusive choice

![SegmentedControl](img/SegmentedControl.png)

Open live: `cargo run -p storybook --features macos -- --select SegmentedControl`

### Switch

*Choice* — On/off toggle

![Switch](img/Switch.png)

Open live: `cargo run -p storybook --features macos -- --select Switch`

### ColorWell

*Choice* — Colour picker swatch

![ColorWell](img/ColorWell.png)

Open live: `cargo run -p storybook --features macos -- --select ColorWell`

### DatePicker

*Pickers* — Calendar date

![DatePicker](img/DatePicker.png)

Open live: `cargo run -p storybook --features macos -- --select DatePicker`

### TimePicker

*Pickers* — Time of day

![TimePicker](img/TimePicker.png)

Open live: `cargo run -p storybook --features macos -- --select TimePicker`

### GraphicalTimePicker

*Pickers* — Clock-face time picker

![GraphicalTimePicker](img/GraphicalTimePicker.png)

Open live: `cargo run -p storybook --features macos -- --select GraphicalTimePicker`

### Sidebar

*Lists* — Navigation rail

![Sidebar](img/Sidebar.png)

Open live: `cargo run -p storybook --features macos -- --select Sidebar`

### SidebarItem

*Lists* — A sidebar row

![SidebarItem](img/SidebarItem.png)

Open live: `cargo run -p storybook --features macos -- --select SidebarItem`

### SidebarSection

*Lists* — Group header in a sidebar

![SidebarSection](img/SidebarSection.png)

Open live: `cargo run -p storybook --features macos -- --select SidebarSection`

### ListTile

*Lists* — Settings-style row

![ListTile](img/ListTile.png)

Open live: `cargo run -p storybook --features macos -- --select ListTile`

### GroupedSection

*Lists* — Grouped rows with dividers

![GroupedSection](img/GroupedSection.png)

Open live: `cargo run -p storybook --features macos -- --select GroupedSection`

### FlatList

*Lists* — Virtualised list

![FlatList](img/FlatList.png)

Open live: `cargo run -p storybook --features macos -- --select FlatList`

### ScrollView

*Lists* — Scrollable region

![ScrollView](img/ScrollView.png)

Open live: `cargo run -p storybook --features macos -- --select ScrollView`

### TouchableOpacity

*Lists* — Pressing container

![TouchableOpacity](img/TouchableOpacity.png)

Open live: `cargo run -p storybook --features macos -- --select TouchableOpacity`

### LayoutContainer

*Layout* — Stacking container

![LayoutContainer](img/LayoutContainer.png)

Open live: `cargo run -p storybook --features macos -- --select LayoutContainer`

### Card

*Layout* — Grouped content surface

![Card](img/Card.png)

Open live: `cargo run -p storybook --features macos -- --select Card`

### ContentArea

*Layout* — Main window pane

![ContentArea](img/ContentArea.png)

Open live: `cargo run -p storybook --features macos -- --select ContentArea`

### Scaffold

*Layout* — Window structure

![Scaffold](img/Scaffold.png)

Open live: `cargo run -p storybook --features macos -- --select Scaffold`

### SplitView

*Layout* — Resizable panes

![SplitView](img/SplitView.png)

Open live: `cargo run -p storybook --features macos -- --select SplitView`

### TabView

*Layout* — Native tabbed panes

![TabView](img/TabView.png)

Open live: `cargo run -p storybook --features macos -- --select TabView`

### TabItem

*Layout* — One page of a TabView

![TabItem](img/TabItem.png)

Open live: `cargo run -p storybook --features macos -- --select TabItem`

### SafeAreaView

*Layout* — Inset container

![SafeAreaView](img/SafeAreaView.png)

Open live: `cargo run -p storybook --features macos -- --select SafeAreaView`

### SearchBar

*Layout* — Search field

![SearchBar](img/SearchBar.png)

Open live: `cargo run -p storybook --features macos -- --select SearchBar`

### Window

*Chrome* — Root window

![Window](img/Window.png)

Open live: `cargo run -p storybook --features macos -- --select Window`

### TitleBar

*Chrome* — Window title strip

![TitleBar](img/TitleBar.png)

Open live: `cargo run -p storybook --features macos -- --select TitleBar`

### Toolbar

*Chrome* — Action bar

![Toolbar](img/Toolbar.png)

Open live: `cargo run -p storybook --features macos -- --select Toolbar`

### ToolbarIconButton

*Chrome* — Toolbar icon action

![ToolbarIconButton](img/ToolbarIconButton.png)

Open live: `cargo run -p storybook --features macos -- --select ToolbarIconButton`

### ToolbarSpacer

*Chrome* — Flexible gap

![ToolbarSpacer](img/ToolbarSpacer.png)

Open live: `cargo run -p storybook --features macos -- --select ToolbarSpacer`

### ToolbarDivider

*Chrome* — Rule between actions

![ToolbarDivider](img/ToolbarDivider.png)

Open live: `cargo run -p storybook --features macos -- --select ToolbarDivider`

### ToolbarPopup

*Chrome* — Toolbar pull-down

![ToolbarPopup](img/ToolbarPopup.png)

Open live: `cargo run -p storybook --features macos -- --select ToolbarPopup`

### StatusBar

*Chrome* — Bottom status strip

![StatusBar](img/StatusBar.png)

Open live: `cargo run -p storybook --features macos -- --select StatusBar`

### IconButton

*Buttons* — Icon-only button

![IconButton](img/IconButton.png)

Open live: `cargo run -p storybook --features macos -- --select IconButton`

### BackButton

*Buttons* — Navigation chevron

![BackButton](img/BackButton.png)

Open live: `cargo run -p storybook --features macos -- --select BackButton`

### DisclosureButton

*Buttons* — Show/hide toggle

![DisclosureButton](img/DisclosureButton.png)

Open live: `cargo run -p storybook --features macos -- --select DisclosureButton`

### HelpButton

*Buttons* — Contextual help

![HelpButton](img/HelpButton.png)

Open live: `cargo run -p storybook --features macos -- --select HelpButton`

### RichText

*Text* — Mixed formatting

![RichText](img/RichText.png)

Open live: `cargo run -p storybook --features macos -- --select RichText`

### Tooltip

*Text* — Hint on a control

![Tooltip](img/Tooltip.png)

Open live: `cargo run -p storybook --features macos -- --select Tooltip`

### Modal

*Dialogs* — Blocking overlay

![Modal](img/Modal.png)

Open live: `cargo run -p storybook --features macos -- --select Modal`

### Popover

*Dialogs* — Anchored overlay

![Popover](img/Popover.png)

Open live: `cargo run -p storybook --features macos -- --select Popover`

### AlertDialog

*Dialogs* — Alert with choice

![AlertDialog](img/AlertDialog.png)

Open live: `cargo run -p storybook --features macos -- --select AlertDialog`

### Sheet

*Dialogs* — Window-attached modal

![Sheet](img/Sheet.png)

Open live: `cargo run -p storybook --features macos -- --select Sheet`

