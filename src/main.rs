use gtk::{
    ButtonExt, ContainerExt, EntryExt, GestureDragExt, GtkWindowExt, Inhibit, LabelExt, OverlayExt,
    WidgetExt, Window,
};
use relm::{connect, Relm, Update, Widget};
use relm_derive::Msg;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};

const PATHCOLOR: (f64, f64, f64, f64) = (0.105, 0.117, 0.746, 0.9);
const PATHWIDTH: f64 = 4.5;

fn main() {
    Win::run(()).expect("Win::run failed");
}

struct Model {
    path: Vec<(f64, f64)>,
    path_rel: Vec<(f64, f64)>,
    words: HashSet<String>,
    word_paths: HashMap<String, Vec<(f64, f64)>>,
}

#[derive(Msg)]
enum Msg {
    StartGesture(f64, f64),
    UpdateGesture(f64, f64),
    EndGesture,
    Search(String),
    Quit,
}

struct Matches {
    sug_1: gtk::Button,
    sug_2: gtk::Button,
    sug_3: gtk::Button,
    sug_4: gtk::Button,
    sug_5: gtk::Button,
    sug_6: gtk::Button,
    sug_7: gtk::Button,
}

struct Win {
    pub relm: relm::Relm<Win>,
    window: Window,
    overlay: gtk::Overlay,
    draw_handler: relm::DrawHandler<gtk::DrawingArea>,
    search_entry: gtk::SearchEntry,
    search_label: gtk::Label,
    matches: Matches,
    model: Model,
    drag_gesture: gtk::GestureDrag,
}

impl Update for Win {
    // Specify the model used for this widget.
    type Model = Model;
    // Specify the model parameter used to init the model.
    type ModelParam = ();
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_: &Relm<Self>, _: ()) -> Model {
        let path = Vec::new();
        let path_rel = Vec::new();
        let words = get_word_list("word_list.txt");
        let mut word_paths = HashMap::new();
        for word in &words {
            word_paths.insert(word.clone(), path_gen::get_path(&word));
        }
        Model {
            path,
            path_rel,
            words,
            word_paths,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::StartGesture(x, y) => {
                self.model.path.push((x, y));
                let (x, y) = self.get_rel_coordinates(x, y);
                self.model.path_rel.push((x, y));
            }
            Msg::UpdateGesture(x, y) => {
                self.model.path.push((x, y));
                let (x, y) = self.get_rel_coordinates(x, y);
                self.model.path_rel.push((x, y));
                self.draw_path();
            }
            Msg::EndGesture => {
                self.find_similar_words();
                self.model.path = Vec::new();
                self.model.path_rel = Vec::new();
                self.erase_path();
            }
            Msg::Search(text) => {
                if self.model.words.contains(&text) {
                    self.search_label.set_text("         Yes         ");
                } else {
                    self.search_label.set_text("         No          ");
                }
            }
            Msg::Quit => gtk::main_quit(),
        }
    }
}

impl Widget for Win {
    // Specify the type of the root widget.
    type Root = Window;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.window.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        // Load the screenshot of the keyboard to load as a background
        let filename = "key_layout.png";

        // Conditionally compile this part
        // For the Pinephone the picture would be too large, so it needs to be scaled down
        let key_grid = if cfg!(target_arch = "aarch64") {
            let width = 300;
            let height = 200;
            let preserve_aspect_ratio = true;

            let screenshot = gdk_pixbuf::Pixbuf::from_file_at_scale(
                filename,
                width,
                height,
                preserve_aspect_ratio,
            )
            .unwrap();

            gtk::Image::from_pixbuf(Some(&screenshot))
        } else {
            gtk::Image::from_file(filename)
        };

        key_grid.get_preferred_width();

        let drawing_area = gtk::DrawingArea::new();
        let mut draw_handler = relm::DrawHandler::new().expect("draw handler");
        draw_handler.init(&drawing_area);

        // Make search entry
        let search_entry = gtk::SearchEntry::new();
        search_entry.set_hexpand(true);
        let search_label = gtk::Label::new(Some("      ---------      "));
        // search_label.width_request(100);
        let h_box = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        h_box.set_hexpand(true);
        h_box.add(&search_entry);
        h_box.add(&search_label);

        // Overlay the drawing area over the button grid
        let overlay = gtk::Overlay::new();
        overlay.add(&key_grid);
        overlay.add_overlay(&drawing_area);

        // Make vertical box with closest matches
        let v_box_matches = gtk::Box::new(gtk::Orientation::Vertical, 2);
        let sug_1 = gtk::Button::new();
        let sug_2 = gtk::Button::new();
        let sug_3 = gtk::Button::new();
        let sug_4 = gtk::Button::new();
        let sug_5 = gtk::Button::new();
        let sug_6 = gtk::Button::new();
        let sug_7 = gtk::Button::new();
        v_box_matches.add(&sug_1);
        v_box_matches.add(&sug_2);
        v_box_matches.add(&sug_3);
        v_box_matches.add(&sug_4);
        v_box_matches.add(&sug_5);
        v_box_matches.add(&sug_6);
        v_box_matches.add(&sug_7);
        let matches = Matches {
            sug_1,
            sug_2,
            sug_3,
            sug_4,
            sug_5,
            sug_6,
            sug_7,
        };

        // Make the vertical box that stores the "keyboard to draw gestures" and the text input to look up if a word is in the word list
        let v_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
        v_box.add(&h_box);
        v_box.add(&overlay);
        v_box.add(&v_box_matches);

        // Add a GestureDrag handler to the drawing area
        let drag_gesture = gtk::GestureDrag::new(&drawing_area);

        // Make the window that contains the UI
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_property_default_height(720);
        window.add(&v_box);

        window.show_all();

        Win {
            relm: relm.clone(),
            window,
            overlay,
            draw_handler,
            search_entry,
            search_label,
            matches,
            model,
            drag_gesture,
        }
    }
    /// Initialize the view
    /// This includes adding callbacks for GTK events and starting the UI with the currently active layout/view
    fn init_view(&mut self) {
        // Send a 'GestureSignal' message to the UI with the coordinates and a GestureSignal::DragBegin variant when the beginning of a drag was detected on the overlay
        relm::connect!(
            self.drag_gesture,
            connect_drag_begin(_, x, y),
            self.relm,
            Msg::StartGesture(x, y)
        );

        // Send a 'GestureSignal' message to the UI with the coordinates and a GestureSignal::DragUpdate variant when a drag was already detected
        // on the overlay and the finger was moved was
        relm::connect!(
            self.drag_gesture,
            connect_drag_update(drag_gesture, x_offset, y_offset),
            self.relm,
            {
                let (x_start, y_start) =
                    drag_gesture.get_start_point().unwrap_or((-1000.0, -1000.0)); // When popup is opened, there is no startpoint. To avoid being close to any buttons this large negative number is given
                let x = x_start + x_offset;
                let y = y_start + y_offset;
                Msg::UpdateGesture(x, y)
            }
        );

        relm::connect!(self.search_entry, connect_activate(x), self.relm, {
            Msg::Search(x.get_text().to_string())
        });

        // Send a 'GestureSignal' message to the UI with the coordinates and a GestureSignal::DragEnd variant when a drag was already detected
        // on the overlay and the finger was lifted off the screen
        relm::connect!(self.drag_gesture, connect_drag_end(_, _, _), self.relm, {
            Msg::EndGesture
        });

        connect!(
            self.relm,
            self.window,
            connect_delete_event(_, _),
            return (Some(Msg::Quit), Inhibit(false))
        );
    }
}

impl Win {
    fn get_rel_coordinates(&self, x: f64, y: f64) -> (f64, f64) {
        // Get width and height of the gtk::Stack that is used to display the button rows
        let allocation = self.overlay.get_allocation();
        let (width, height) = (allocation.width, allocation.height);
        // Calculate the relative coordinates
        let x_rel = x / width as f64;
        let y_rel = y / height as f64;
        (x_rel, y_rel)
    }

    /// Erases the path/gesture the user drew
    fn erase_path(&mut self) {
        let context = self.draw_handler.get_context();
        context.set_operator(cairo::Operator::Clear);
        context.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        context.paint();
    }

    /// Paint the path/gesture the user drew
    fn draw_path(&mut self) {
        // Delete the previous path
        self.erase_path();
        // Set path colors
        let context = self.draw_handler.get_context();
        context.set_operator(cairo::Operator::Over);
        context.set_source_rgba(PATHCOLOR.0, PATHCOLOR.1, PATHCOLOR.2, PATHCOLOR.3);
        // Get the dots and connect them with a line
        for dot in &self.model.path {
            // Create a line between the previous dot and the current one
            context.line_to(dot.0, dot.1);
        }
        context.set_line_width(PATHWIDTH);
        // Paint the line of dots
        context.stroke();
    }

    fn find_similar_words(&self) {
        let k = 7;
        let query = &self.model.path_rel;
        let mut dist;
        let mut k_best: Vec<(String, f64)> = vec![(String::new(), f64::INFINITY); k]; // Stores the k nearest neighbors (location, DTW distance)
        let mut bsf = k_best[k - 1].1;

        // Compare the paths of each word
        for (candidate_word, candidate_word_path) in &self.model.word_paths {
            // The cb currently needs to have the same length as the longer sequence for dtw not to panic
            let n_longer_seq = if query.len() < candidate_word_path.len() {
                candidate_word_path.len()
            } else {
                query.len()
            };
            let cb = vec![0.0; n_longer_seq];
            dist = dtw::ucr_improved::dtw(
                &candidate_word_path,
                query,
                &cb,
                n_longer_seq - 1,
                bsf,
                &dist_points,
            );
            if candidate_word == "hello" {
                println!("Path for hello:");
                for (x, y) in candidate_word_path {
                    print!("({:.3}/{:.3})", x, y);
                }
                println!();
            }

            if dist < bsf {
                let candidate: String = candidate_word.to_owned();
                knn_dtw::ucr::insert_into_k_bsf((candidate, dist), &mut k_best);
                bsf = k_best[k - 1].1;
            }
        }

        self.matches.sug_1.set_label(&k_best[0].0);
        self.matches.sug_2.set_label(&k_best[1].0);
        self.matches.sug_3.set_label(&k_best[2].0);
        self.matches.sug_4.set_label(&k_best[3].0);
        self.matches.sug_5.set_label(&k_best[4].0);
        self.matches.sug_6.set_label(&k_best[5].0);
        self.matches.sug_7.set_label(&k_best[6].0);

        println!("Drawn path:");
        for (x, y) in &self.model.path_rel {
            print!("({:.3}/{:.3})", x, y);
        }
        println!();

        println!("Print path for word \"{}\"", k_best[0].0);
        println!("The word had a distance of {}", k_best[0].1);
        let path_best_match = self.model.word_paths.get(&k_best[0].0);
        if let Some(word_path) = path_best_match {
            println!("Best matching path:");
            for (x, y) in word_path {
                print!("({:.3}/{:.3})", x, y);
            }
        } else {
            println!("No best path was found!!");
        }
    }
}

fn dist_points(a: &(f64, f64), b: &(f64, f64)) -> f64 {
    f64::sqrt((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2))
}

pub fn get_word_list(filename: &str) -> HashSet<String> {
    // Open the file in read-only mode.
    let file = File::open(filename).unwrap();
    let buf_reader = BufReader::new(file);
    let mut words = HashSet::new();
    for word in buf_reader.lines() {
        if let Ok(word) = word {
            words.insert(word);
        }
    }
    words
}
