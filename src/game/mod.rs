mod entry;
mod gui;
mod stack;
mod stages;

use entry::Entry;
use stack::kind::Kind;
use stack::Stack;
use std::collections::HashMap;

pub struct Game {
    stacks: Vec<Stack>,
    units_per_kind: HashMap<Kind, usize>,
    kind_indices: HashMap<Kind, usize>,
    kinds_status: usize,
    turn: usize,
    stage_name: String,
    ledger: Vec<Entry>,
}

impl Game {
    fn new(stacks: Vec<Stack>, stage_name: Option<String>) -> Game {
        let units_per_kind: HashMap<Kind, usize> = Game::count_kinds(&stacks);
        let kind_indices: HashMap<Kind, usize> = Game::index_kinds(&units_per_kind);
        Game {
            stacks,
            units_per_kind,
            kind_indices,
            kinds_status: 0,
            turn: 1,
            stage_name: stage_name.unwrap_or("".to_string()),
            ledger: Vec::new(),
        }
    }

    fn clone(&self) -> Game {
        Game::new(
            self.stacks.iter().map(|stack| stack.clone()).collect(),
            Some(self.stage_name.clone()),
        )
    }

    fn count_kinds(stacks: &[Stack]) -> HashMap<Kind, usize> {
        let mut units_per_kind: HashMap<Kind, usize> = HashMap::new(); // Initialize the HashMap
        for stack in stacks {
            for unit in &stack.units {
                *units_per_kind.entry(*unit).or_insert(0) += 1; // Populate the HashMap
            }
        }
        units_per_kind
    }

    fn index_kinds(units_per_kind: &HashMap<Kind, usize>) -> HashMap<Kind, usize> {
        let mut kind_indices: HashMap<Kind, usize> = HashMap::new();
        let mut kinds: Vec<&Kind> = units_per_kind.keys().collect();
        kinds.sort(); // Sort kinds by their id
        for (index, kind) in kinds.iter().enumerate() {
            kind_indices.insert(**kind, index);
        }
        kind_indices
    }

    fn move_is_legal(&self, immigrants: &Stack, residents: &Stack) -> bool {
        let top_immigrant: Kind = immigrants.clone_top_unit();
        let top_resident: Kind = residents.clone_top_unit();
        let tops_match: bool =
            (top_immigrant == top_resident) || top_immigrant.is_empty() || top_resident.is_empty();
        let there_is_room: bool = immigrants.units.len() <= residents.get_vacancy();
        tops_match && there_is_room
    }

    fn update_kind_status(&mut self, stack_ind: usize) {
        let immigrants: &mut Stack = &mut Stack::new();

        self.stacks[stack_ind].pop_immigrants(immigrants);
        let top_immigrant: Kind = immigrants.clone_top_unit();
        if !top_immigrant.is_empty() {
            let kind_status_operand: usize = 1 << self.kind_indices[&top_immigrant];
            self.kinds_status |= kind_status_operand; // Initially set the kth bit to 1.
            if immigrants.units.len() != self.units_per_kind[&top_immigrant] {
                self.kinds_status -= kind_status_operand;  // zero the kth bit.
            }
        }
        self.stacks[stack_ind].push_immigrants(immigrants);
    }

    fn ledge(&mut self, from: usize, to: usize, kind: Kind, quantity: usize) {
        self.ledger.push(Entry {
            from,
            to,
            _kind: kind,
            quantity,
        });
    }

    fn update_state(&mut self, from: usize, to: usize) {
        self.update_kind_status(from);
        self.update_kind_status(to);
        self.turn += if self.stage_complete() { 0 } else { 1 };
    }

    fn move_units(&mut self, from: usize, to: usize, limit_: Option<usize>) {
        let immigrants: &mut Stack = &mut Stack::new();
        self.stacks[from].pop_immigrants_with_limit(immigrants, limit_);
        let kind: Kind =  immigrants.clone_top_unit();
        let quantity: usize = immigrants.units.len();
        let move_approved: bool =
            limit_.is_some() || self.move_is_legal(&immigrants, &self.stacks[to]); // TODO: illegal moves should prompt users for new input.
        let dest: usize = if move_approved { to } else { from };
        self.stacks[dest].push_immigrants(immigrants);
        
        if move_approved {
            self.update_state(from, to);
            match limit_ {
                Some(_) => {} // When a limit is specified this is an undo move, it should not be ledged.
                _ => self.ledge(
                    from,
                    to,
                    kind,
                    quantity,
                ),
            };
        }
    }

    fn move_legally(&mut self, from: usize, to: usize) {
        self.move_units(from, to, None);
    }

    fn move_forcefully(&mut self, from: usize, to: usize, quantity: usize) {
        self.move_units(from, to, Some(quantity));
    }

    fn stage_complete(&self) -> bool {
        self.kinds_status == (1 << self.units_per_kind.len()) - 1
    }

    fn undo_move(&mut self) {
        match self.ledger.last() {
            Some(entry) => {
                let (from, to, quantity) = (entry.to, entry.from, entry.quantity);
                self.ledger.pop();
                self.move_forcefully(from, to, quantity);
            }
            _ => {}  // No moves to undo.
        }
    }

    fn turn_loop(&mut self) {
        let stage_backup: Game = self.clone();
        loop {
            self.render();
            if self.stage_complete() {
                break;
            }
            let user_input: gui::UserInput = self.read_valid_input();
            match user_input.stack_move {
                Some((from, to)) => self.move_legally(from, to),
                _ => match user_input.menu_option {
                    gui::MenuOption::Reset => *self = stage_backup.clone(),
                    gui::MenuOption::Undo => self.undo_move(),
                    _ => {} // TODO: implement Help and Quit cases.
                },
            }
        }
    }

    pub fn play() {
        let stages: Vec<Game> = Game::get_stages();
        let last_stage_index: usize = stages.len() - 1;
        for (ind, mut stage) in stages.into_iter().enumerate() {
            stage.turn_loop();
            stage.stage_complete_prompt(ind == last_stage_index);
        }
    }
}
