#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("This is the WASM web client. Run `trunk serve` inside tetris-wasm/web_client to play.");
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use game_core::{Game, Step, BOARD_H, BOARD_W};
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, KeyboardEvent};

    console_error_panic_hook::set_once();

    const CELL: f64 = 30.0;

    fn color_for(id: u8) -> &'static str {
        match id {
            1 => "#00f0f0", // I
            2 => "#f0f000", // O
            3 => "#a000f0", // T
            4 => "#00f000", // S
            5 => "#f00000", // Z
            6 => "#0000f0", // J
            7 => "#f0a000", // L
            _ => "#000000",
        }
    }

    fn draw_cell(ctx: &CanvasRenderingContext2d, x: i32, y: i32, id: u8) {
        draw_cell_alpha(ctx, x, y, id, 1.0);
    }

    fn draw_cell_alpha(ctx: &CanvasRenderingContext2d, x: i32, y: i32, id: u8, alpha: f64) {
        let fx = (x as f64) * CELL;
        let fy = (y as f64) * CELL;

        ctx.save();
        ctx.set_global_alpha(alpha);
        ctx.set_fill_style_str(color_for(id));
        ctx.fill_rect(fx + 1.0, fy + 1.0, CELL - 2.0, CELL - 2.0);

        ctx.set_stroke_style_str("#1a1a1a");
        ctx.stroke_rect(fx + 0.5, fy + 0.5, CELL - 1.0, CELL - 1.0);
        ctx.restore();
    }

    const SHAPES: [[[(i8, i8); 4]; 4]; 7] = [
        [
            [(0, 1), (1, 1), (2, 1), (3, 1)],
            [(2, 0), (2, 1), (2, 2), (2, 3)],
            [(0, 2), (1, 2), (2, 2), (3, 2)],
            [(1, 0), (1, 1), (1, 2), (1, 3)],
        ],
        [
            [(1, 0), (2, 0), (1, 1), (2, 1)],
            [(1, 0), (2, 0), (1, 1), (2, 1)],
            [(1, 0), (2, 0), (1, 1), (2, 1)],
            [(1, 0), (2, 0), (1, 1), (2, 1)],
        ],
        [
            [(1, 0), (0, 1), (1, 1), (2, 1)],
            [(1, 0), (1, 1), (2, 1), (1, 2)],
            [(0, 1), (1, 1), (2, 1), (1, 2)],
            [(1, 0), (0, 1), (1, 1), (1, 2)],
        ],
        [
            [(1, 0), (2, 0), (0, 1), (1, 1)],
            [(1, 0), (1, 1), (2, 1), (2, 2)],
            [(1, 1), (2, 1), (0, 2), (1, 2)],
            [(0, 0), (0, 1), (1, 1), (1, 2)],
        ],
        [
            [(0, 0), (1, 0), (1, 1), (2, 1)],
            [(2, 0), (1, 1), (2, 1), (1, 2)],
            [(0, 1), (1, 1), (1, 2), (2, 2)],
            [(1, 0), (0, 1), (1, 1), (0, 2)],
        ],
        [
            [(0, 0), (0, 1), (1, 1), (2, 1)],
            [(1, 0), (2, 0), (1, 1), (1, 2)],
            [(0, 1), (1, 1), (2, 1), (2, 2)],
            [(1, 0), (1, 1), (0, 2), (1, 2)],
        ],
        [
            [(2, 0), (0, 1), (1, 1), (2, 1)],
            [(1, 0), (1, 1), (1, 2), (2, 2)],
            [(0, 1), (1, 1), (2, 1), (0, 2)],
            [(0, 0), (1, 0), (1, 1), (1, 2)],
        ],
    ];

    fn blocks_for_piece(p: game_core::Piece) -> [(i32, i32); 4] {
        let kind_idx = (p.kind.id() as usize) - 1;
        let rot_idx = (p.rot % 4) as usize;
        let shape = &SHAPES[kind_idx][rot_idx];
        let mut out = [(0i32, 0i32); 4];
        for (i, (dx, dy)) in shape.iter().enumerate() {
            out[i] = (p.x + (*dx as i32), p.y + (*dy as i32));
        }
        out
    }

    fn sfx(window: &web_sys::Window, name: &str) {
        let val = js_sys::Reflect::get(window, &wasm_bindgen::JsValue::from_str("tetrisSfx"));
        if let Ok(v) = val {
            if let Some(f) = v.dyn_ref::<js_sys::Function>() {
                let _ = f.call1(window, &wasm_bindgen::JsValue::from_str(name));
            }
        }
    }

    fn apply_step_sfx(window: &web_sys::Window, step: Step) {
        match step {
            Step::Moved => {}
            Step::Locked { cleared, game_over } => {
                sfx(window, "drop");
                if cleared > 0 {
                    sfx(window, "line");
                }
                if game_over {
                    sfx(window, "gameover");
                }
            }
            Step::GameOver => {}
        }
    }

    fn render(ctx: &CanvasRenderingContext2d, game: &Game) {
        // Clear
        ctx.set_fill_style_str("#111111");
        ctx.fill_rect(0.0, 0.0, (BOARD_W as f64) * CELL, (BOARD_H as f64) * CELL);

        // Locked cells
        for y in 0..BOARD_H {
            for x in 0..BOARD_W {
                let id = game.cell(x, y);
                if id != 0 {
                    draw_cell(ctx, x, y, id);
                }
            }
        }

        // Ghost piece
        let ghost = game.ghost_piece();
        for (x, y) in blocks_for_piece(ghost) {
            if y >= 0 && y < BOARD_H {
                draw_cell_alpha(ctx, x, y, ghost.kind.id(), 0.22);
            }
        }

        // Current piece
        let p = game.current_piece();
        for (x, y) in blocks_for_piece(p) {
            if y >= 0 && y < BOARD_H {
                draw_cell(ctx, x, y, p.kind.id());
            }
        }

        if game.is_game_over() {
            ctx.set_fill_style_str("rgba(0,0,0,0.65)");
            ctx.fill_rect(0.0, 0.0, (BOARD_W as f64) * CELL, (BOARD_H as f64) * CELL);

            ctx.set_fill_style_str("#ffffff");
            ctx.set_font("bold 28px sans-serif");
            ctx.fill_text("GAME OVER", 35.0, 260.0).ok();
            ctx.set_font("16px sans-serif");
            ctx.fill_text("Press R to restart", 55.0, 290.0).ok();
        }
    }

    fn drop_interval_ms(level: u32) -> f64 {
        // Very simple speed curve.
        let base = 650.0;
        let step = 45.0;
        let lvl = (level.saturating_sub(1)).min(12) as f64;
        (base - step * lvl).max(80.0)
    }

    let window = web_sys::window().expect("no window");
    let window_for_raf = window.clone();
    let document = window.document().expect("no document");

    let canvas: HtmlCanvasElement = document
        .get_element_by_id("tetris")
        .expect("missing <canvas id=\"tetris\">")
        .dyn_into()
        .expect("tetris element is not a canvas");

    canvas.set_width(((BOARD_W as f64) * CELL) as u32);
    canvas.set_height(((BOARD_H as f64) * CELL) as u32);

    let ctx: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into()
        .unwrap();

    let hud = document
        .get_element_by_id("hud")
        .expect("missing #hud");

    let game = Rc::new(RefCell::new(Game::new()));
    let paused = Rc::new(RefCell::new(false));

    // Keyboard controls
    {
        let game = game.clone();
        let paused = paused.clone();
        let window = window.clone();
        let keydown = Closure::wrap(Box::new(move |e: KeyboardEvent| {
            if e.repeat() {
                return;
            }
            match e.key().as_str() {
                "ArrowLeft" => {
                    game.borrow_mut().move_left();
                    sfx(&window, "move");
                }
                "ArrowRight" => {
                    game.borrow_mut().move_right();
                    sfx(&window, "move");
                }
                "ArrowDown" => {
                    let step = game.borrow_mut().soft_drop();
                    apply_step_sfx(&window, step);
                }
                "ArrowUp" | "x" | "X" => {
                    game.borrow_mut().rotate_cw();
                    sfx(&window, "rotate");
                }
                " " => {
                    let step = game.borrow_mut().hard_drop();
                    apply_step_sfx(&window, step);
                }
                "p" | "P" => *paused.borrow_mut() = !*paused.borrow(),
                "r" | "R" => game.borrow_mut().reset(),
                _ => {}
            }
        }) as Box<dyn FnMut(_)>);
        document
            .add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref())
            .unwrap();
        keydown.forget();
    }

    // Mobile / on-screen controls
    {
        let doc = document.clone();
        let window0 = window.clone();
        let game0 = game.clone();
        let paused0 = paused.clone();

        let bind = |id: &str, f: Closure<dyn FnMut(web_sys::Event)>| {
            if let Some(el) = doc.get_element_by_id(id) {
                let _ = el.add_event_listener_with_callback("click", f.as_ref().unchecked_ref());
                f.forget();
            }
        };

        {
            let window = window0.clone();
            let game = game0.clone();
            bind(
                "btn-left",
                Closure::wrap(Box::new(move |_e: web_sys::Event| {
                    game.borrow_mut().move_left();
                    sfx(&window, "move");
                }) as Box<dyn FnMut(_)>),
            );
        }

        {
            let window = window0.clone();
            let game = game0.clone();
            bind(
                "btn-right",
                Closure::wrap(Box::new(move |_e: web_sys::Event| {
                    game.borrow_mut().move_right();
                    sfx(&window, "move");
                }) as Box<dyn FnMut(_)>),
            );
        }

        {
            let window = window0.clone();
            let game = game0.clone();
            bind(
                "btn-rotate",
                Closure::wrap(Box::new(move |_e: web_sys::Event| {
                    game.borrow_mut().rotate_cw();
                    sfx(&window, "rotate");
                }) as Box<dyn FnMut(_)>),
            );
        }

        {
            let window = window0.clone();
            let game = game0.clone();
            bind(
                "btn-down",
                Closure::wrap(Box::new(move |_e: web_sys::Event| {
                    let step = game.borrow_mut().soft_drop();
                    apply_step_sfx(&window, step);
                }) as Box<dyn FnMut(_)>),
            );
        }

        {
            let window = window0.clone();
            let game = game0.clone();
            bind(
                "btn-drop",
                Closure::wrap(Box::new(move |_e: web_sys::Event| {
                    let step = game.borrow_mut().hard_drop();
                    apply_step_sfx(&window, step);
                }) as Box<dyn FnMut(_)>),
            );
        }

        {
            let paused = paused0.clone();
            bind(
                "btn-pause",
                Closure::wrap(Box::new(move |_e: web_sys::Event| {
                    *paused.borrow_mut() = !*paused.borrow();
                }) as Box<dyn FnMut(_)>),
            );
        }

        {
            let game = game0.clone();
            bind(
                "btn-restart",
                Closure::wrap(Box::new(move |_e: web_sys::Event| {
                    game.borrow_mut().reset();
                }) as Box<dyn FnMut(_)>),
            );
        }
    }

    // Animation loop
    let last_time = Rc::new(RefCell::new(0.0f64));
    let drop_acc = Rc::new(RefCell::new(0.0f64));

    let raf_cb: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let raf_cb2 = raf_cb.clone();

    let game2 = game.clone();
    let paused2 = paused.clone();

    *raf_cb2.borrow_mut() = Some(Closure::wrap(Box::new(move |t: f64| {
        let mut last = last_time.borrow_mut();
        let dt = if *last == 0.0 { 0.0 } else { t - *last };
        *last = t;

        if !*paused2.borrow() {
            let lvl = game2.borrow().level();
            *drop_acc.borrow_mut() += dt;
            let interval = drop_interval_ms(lvl);
            while *drop_acc.borrow() >= interval {
                *drop_acc.borrow_mut() -= interval;
                let step = game2.borrow_mut().tick();
                apply_step_sfx(&window_for_raf, step);
            }
        }

        {
            let g = game2.borrow();
            render(&ctx, &g);
            hud.set_inner_html(&format!(
                "<div><b>Controls</b>: ← → ↓ rotate: ↑/X drop: Space pause: P restart: R</div>\
                 <div><b>Score</b>: {} &nbsp;&nbsp; <b>Lines</b>: {} &nbsp;&nbsp; <b>Level</b>: {}{}</div>",
                g.score(),
                g.lines(),
                g.level(),
                if *paused2.borrow() { " &nbsp;&nbsp; <b>PAUSED</b>" } else { "" }
            ));
        }

        window_for_raf
            .request_animation_frame(
                raf_cb.borrow().as_ref().unwrap().as_ref().unchecked_ref(),
            )
            .unwrap();
    }) as Box<dyn FnMut(f64)>));

    window
        .request_animation_frame(
            raf_cb2.borrow().as_ref().unwrap().as_ref().unchecked_ref(),
        )
        .unwrap();
}
