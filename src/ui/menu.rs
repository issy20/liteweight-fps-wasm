//! Keyboard selection state for the pause menu.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PauseMenuAction {
    Resume,
    ChangeMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TitleMenuAction {
    FreeTraining,
    TrackingAim,
    Settings,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TitleSelection {
    selected: usize,
}

impl TitleSelection {
    const ITEM_COUNT: usize = 3;

    pub fn selected_index(self) -> usize {
        self.selected
    }

    pub fn move_up(&mut self) {
        self.selected = (self.selected + Self::ITEM_COUNT - 1) % Self::ITEM_COUNT;
    }

    pub fn move_down(&mut self) {
        self.selected = (self.selected + 1) % Self::ITEM_COUNT;
    }

    pub fn activate(self) -> TitleMenuAction {
        match self.selected {
            0 => TitleMenuAction::FreeTraining,
            1 => TitleMenuAction::TrackingAim,
            _ => TitleMenuAction::Settings,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PauseSelection {
    selected: usize,
}

impl PauseSelection {
    const ITEM_COUNT: usize = 18;

    pub fn selected_index(self) -> usize {
        self.selected
    }

    pub fn move_up(&mut self) {
        self.selected = (self.selected + Self::ITEM_COUNT - 1) % Self::ITEM_COUNT;
    }

    pub fn move_down(&mut self) {
        self.selected = (self.selected + 1) % Self::ITEM_COUNT;
    }

    pub fn setting_index(self) -> Option<usize> {
        match self.selected {
            1 => Some(0),
            2 => Some(1),
            3 => Some(2),
            6 => Some(9),
            8 => Some(10),
            10 => Some(5),
            11 => Some(11),
            12 => Some(3),
            13 => Some(4),
            14 => Some(6),
            15 => Some(7),
            16 => Some(8),
            _ => None,
        }
    }

    pub fn toggle_setting_index(self) -> Option<usize> {
        match self.selected {
            4 => Some(2),
            7 => Some(1),
            9 => Some(0),
            _ => None,
        }
    }

    pub fn activate(self) -> Option<PauseMenuAction> {
        match self.selected {
            0 => Some(PauseMenuAction::Resume),
            17 => Some(PauseMenuAction::ChangeMode),
            _ => None,
        }
    }
}
