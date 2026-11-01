#[macro_use]
extern crate tracing;

use std::{
    error::Error,
    path::{Path, PathBuf},
};

use serde::Serialize;

pub struct Build {
    name: String,
    comment: String,
    srcdir: PathBuf,
    outdir: PathBuf,
    colorscheme: ColorScheme,
}

pub type Metadata = Vec<Frame>;

#[derive(Serialize, Debug)]
pub struct Frame {
    pub filename: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<u32>,
    pub hotspot_x: u32,
    pub hotspot_y: u32,
    pub nominal_size: u32,
}

pub fn validate_or_panic(metadata: &Metadata) {
    assert!(metadata.len() > 0, "cursor has no frames");
    if metadata.len() > 1 {
        let all_frames_have_a_delay = metadata.iter().all(|f| f.delay.is_some());
        assert!(all_frames_have_a_delay);

        let all_frames_have_a_positive_delay =
            metadata.iter().all(|f| f.delay.is_some_and(|d| d > 0));
        assert!(all_frames_have_a_positive_delay);
    }
}

pub enum ColorScheme {
    Fuchsia,
    FuchsiaPop,
    FuchsiaRed,
    FuchsiaAmber,
}

impl Build {
    fn build(&self) -> Result<(), Box<dyn Error>> {
        self.create_outdir()?;
        self.create_index()?;
        self.create_scalable_cursors()?;

        Ok(())
    }

    fn create_outdir(&self) -> eyre::Result<()> {
        if !self.outdir.exists() {
            std::fs::create_dir(&self.outdir)?;
        }
        Ok(())
    }

    fn create_index(&self) -> eyre::Result<()> {
        let index_file = self.outdir.join("index.theme");

        std::fs::write(
            index_file,
            format!(
                r#"[Icon Theme]
Name={}
Comment={}"#,
                self.name, self.comment
            ),
        )?;

        Ok(())
    }

    fn create_cursor(
        &self,
        name: &str,
        hotspot_x: Option<u32>,
        hotspot_y: Option<u32>,
        symlinks: Vec<&str>,
    ) -> eyre::Result<()> {
        let cursor_dir = self.scalable_dir().join(name);

        if !cursor_dir.exists() {
            std::fs::create_dir(&cursor_dir)?;
        }

        let hotspot_x = hotspot_x.unwrap_or(128);
        let hotspot_y = hotspot_y.unwrap_or(128);
        let nominal_size = 256;
        let delay = 35;

        let source_files = self.find_source_files(name);

        let mut frames = vec![];
        match source_files.len() {
            0 => panic!("no cursor {name} found, aborting."),
            num_frames => {
                let is_animated = num_frames > 1;
                for source_svg in source_files {
                    let file_name = source_svg.file_name().unwrap().to_str().unwrap();
                    self.copy_with_style(&source_svg, &cursor_dir.join(file_name))?;
                    let frame = Frame {
                        filename: PathBuf::from(".").join(file_name),
                        delay: if is_animated { Some(delay) } else { None },
                        hotspot_x,
                        hotspot_y,
                        nominal_size,
                    };
                    frames.push(frame);
                }
            }
        }

        let metadata = frames;
        self.write_metadata(name, metadata)?;

        for aliased in symlinks {
            let symlink = self.scalable_dir().join(aliased);
            if symlink.is_symlink() {
                std::fs::remove_file(&symlink)?;
            }
            let target = PathBuf::from(".").join(name);
            std::os::unix::fs::symlink(&target, symlink)?;
        }

        Ok(())
    }

    fn scalable_dir(&self) -> PathBuf {
        self.outdir.join("cursors_scalable")
    }

    fn write_metadata(&self, name: &str, metadata: Metadata) -> eyre::Result<()> {
        let cursor_dir = self.scalable_dir().join(name);
        let metadata_file = cursor_dir.join("metadata.json");
        let contents = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(metadata_file, contents)?;
        Ok(())
    }

    fn copy_with_style(&self, source_svg: &Path, destination_svg: &Path) -> eyre::Result<()> {
        let svg = std::fs::read_to_string(source_svg)?;
        let svg_with_style = svg
            .replace("#00FF00", self.colorscheme.color1())
            .replace("#0000FF", self.colorscheme.color2());
        std::fs::write(destination_svg, svg_with_style)?;
        Ok(())
    }

    fn create_scalable_cursors(&self) -> eyre::Result<()> {
        let scalable_dir = self.scalable_dir();

        if !scalable_dir.exists() {
            std::fs::create_dir(scalable_dir)?;
        }

        self.create_cursor("all-scroll", None, None, vec!["fleur", "size_all"])?;
        self.create_cursor(
            "bottom_left_corner",
            None,
            None,
            vec![
                "fcf1c3c7cd4491d801f1e1c78f100000",
                "sw-resize",
                "ne-resize",
                "size_bdiag",
                "nesw-resize",
                "top_right_corner",
                "fd_double_arrow",
            ],
        )?;
        self.create_cursor(
            "bottom_right_corner",
            None,
            None,
            vec![
                "c7088f0f3e6c8088236ef8e1e3e70000",
                "top_left_corner",
                "se-resize",
                "nw-resize",
                "size_fdiag",
                "nwse-resize",
                "bd_double_arrow",
            ],
        )?;
        self.create_cursor("bottom_tee", Some(128), Some(187), vec![])?;
        self.create_cursor("center_ptr", Some(128), Some(18), vec![])?;
        self.create_cursor("context-menu", Some(33), Some(33), vec![])?;
        self.create_cursor(
            "copy",
            Some(33),
            Some(33),
            vec![
                "1081e37283d90000800003c07f3ef6bf",
                "6407b0e94181790501fd1e167b474872",
                "b66166c04f8c3109214a4fbd64a50fc8",
                "dnd-copy",
            ],
        )?;
        self.create_cursor(
            "cross",
            None,
            None,
            vec!["cross_reverse", "diamond_cross", "tcross", "color-picker"],
        )?;
        self.create_cursor(
            "crossed_circle",
            Some(33),
            Some(33),
            vec![
                "03b6e0fcb3499374a867c041f52298f0",
                "not-allowed",
                "forbidden",
                "circle",
            ],
        )?;
        self.create_cursor("crosshair", None, None, vec![])?;
        self.create_cursor("dnd_no_drop", Some(33), Some(33), vec!["no-drop"])?;
        self.create_cursor(
            "dotbox",
            None,
            None,
            vec!["dot_box_mask", "draped_box", "icon", "target"],
        )?;
        self.create_cursor("hand1", None, None, vec!["grab", "openhand"])?;
        self.create_cursor(
            "hand2",
            Some(33),
            Some(33),
            vec![
                "9d800788f1b08800ae810202380a0822",
                "e29285e634086352946a0e7090d73106",
                "pointer",
                "pointing_hand",
            ],
        )?;
        self.create_cursor(
            "left_ptr",
            Some(33),
            Some(33),
            vec!["arrow", "default", "top_left_arrow"],
        )?;
        self.create_cursor(
            "left_ptr_watch",
            Some(32),
            Some(32),
            vec![
                "00000000000000020006000e7e9ffc3f",
                "08e8e1c95fe2fc01f976f1e063a24ccd",
                "3ecb610c1bf2410f44200f48c40d3599",
                "progress",
            ],
        )?;
        self.create_cursor(
            "left_side",
            None,
            None,
            vec!["w-resize", "right_side", "e-resize"],
        )?;
        self.create_cursor("left_tee", Some(70), Some(126), vec![])?;
        self.create_cursor(
            "link",
            Some(33),
            Some(33),
            vec![
                "3085a0e285430894940527032f8b26df",
                "640fb0e74195791501fd1ed57b41487f",
                "a2a266d0498c3104214a47bd64ab0fc8",
                "alias",
                "dnd-link",
            ],
        )?;
        self.create_cursor("ll_angle", Some(87), Some(171), vec![])?;
        self.create_cursor("lr_angle", Some(170), Some(170), vec![])?;
        self.create_cursor(
            "move",
            None,
            None,
            vec![
                "4498f0e0c1937ffe01fd06f973665830",
                "9081237383d90e509aa00f00170e968f",
                "fcf21c00b30f7e3f83fe0dfd12e71cff",
                "grabbing",
                "pointer_move",
                "dnd-move",
                "closedhand",
                "dnd-none",
            ],
        )?;
        self.create_cursor("pencil", Some(55), Some(203), vec!["draft"])?;
        self.create_cursor("plus", None, None, vec!["cell"])?;
        self.create_cursor(
            "question_arrow",
            Some(33),
            Some(33),
            vec![
                "5c6cd98b3f3ebcb1f9c7f1c204630408",
                "d9ce0ab605698f320427677b458ad60b",
                "help",
                "left_ptr_help",
                "whats_this",
                "dnd-ask",
            ],
        )?;
        self.create_cursor(
            "right_ptr",
            Some(223),
            Some(33),
            vec!["draft_large", "draft_small"],
        )?;
        self.create_cursor("right_tee", Some(187), Some(127), vec![])?;
        self.create_cursor("sb_down_arrow", Some(128), Some(191), vec!["down-arrow"])?;
        self.create_cursor(
            "sb_h_double_arrow",
            None,
            None,
            vec![
                "028006030e0e7ebffc7f7070c0600140",
                "14fef782d02440884392942c1120523",
                "col-resize",
                "ew-resize",
                "h_double_arrow",
                "size-hor",
                "size_hor",
                "split_h",
            ],
        )?;
        self.create_cursor("sb_left_arrow", Some(70), Some(128), vec!["left-arrow"])?;
        self.create_cursor("sb_right_arrow", Some(189), Some(128), vec!["right-arrow"])?;
        self.create_cursor("sb_up_arrow", Some(128), Some(68), vec!["up-arrow"])?;
        self.create_cursor(
            "sb_v_double_arrow",
            None,
            None,
            vec![
                "00008160000006810000408080010102",
                "2870a09082c103050810ffdffffe0204",
                "double_arrow",
                "ns-resize",
                "row-resize",
                "size-ver",
                "size_ver",
                "split_v",
                "v_double_arrow",
            ],
        )?;
        self.create_cursor(
            "top_side",
            None,
            None,
            vec!["s-resize", "n-resize", "bottom_side"],
        )?;
        self.create_cursor("top_tee", Some(128), Some(70), vec![])?;
        self.create_cursor("ul_angle", Some(86), Some(86), vec![])?;
        self.create_cursor("ur_angle", Some(188), Some(66), vec![])?;
        self.create_cursor("vertical-text", None, None, vec![])?;
        self.create_cursor("wait", None, None, vec!["watch"])?;
        self.create_cursor("wayland-cursor", None, None, vec![])?;
        self.create_cursor("X_cursor", None, None, vec!["pirate", "x-cursor"])?;
        self.create_cursor("xterm", None, None, vec!["ibeam", "text"])?;
        self.create_cursor("zoom-in", Some(113), Some(113), vec![])?;
        self.create_cursor("zoom-out", Some(113), Some(113), vec![])?;

        Ok(())
    }

    fn find_source_files(&self, name: &str) -> Vec<PathBuf> {
        let dir = self.srcdir.join(name);
        if dir.exists() && dir.is_dir() {
            Vec::from_iter(std::fs::read_dir(dir).unwrap().filter_map(|e| {
                let e = e.ok()?;
                if let Some(_) = e.file_name().to_str().and_then(|f| f.strip_prefix(name)) {
                    Some(e.path())
                } else {
                    None
                }
            }))
        } else {
            let file = self.srcdir.join(name).with_extension("svg");
            if file.exists() && file.is_file() {
                vec![file]
            } else {
                Vec::new()
            }
        }
    }
}

impl ColorScheme {
    pub fn color1(&self) -> &str {
        match self {
            ColorScheme::Fuchsia => "#E11C79",
            ColorScheme::FuchsiaPop => "#F8B572",
            ColorScheme::FuchsiaRed => "#FF0000",
            ColorScheme::FuchsiaAmber => "#FFA400",
        }
    }

    pub fn color2(&self) -> &str {
        match self {
            ColorScheme::Fuchsia => "#FFFFFF",
            ColorScheme::FuchsiaPop => "#FFFFFF",
            ColorScheme::FuchsiaRed => "#FFFFFF",
            ColorScheme::FuchsiaAmber => "#FFFFFF",
        }
    }
}

fn main() {
    tracing_subscriber::fmt::init();
    let builds = [
        Build {
            name: "Fuchsia".to_string(),
            comment: "Scalable Fuchsia Icons".to_string(),
            srcdir: PathBuf::from("../svg"),
            outdir: PathBuf::from(&"Fuchsia"),
            colorscheme: ColorScheme::Fuchsia,
        },
        Build {
            name: "Fuchsia-Amber".to_string(),
            comment: "Scalable Fuchsia Icons (Amber)".to_string(),
            srcdir: PathBuf::from("../svg"),
            outdir: PathBuf::from(&"Fuchsia-Amber"),
            colorscheme: ColorScheme::FuchsiaAmber,
        },
        Build {
            name: "Fuchsia-Pop".to_string(),
            comment: "Scalable Fuchsia Icons (Pop)".to_string(),
            srcdir: PathBuf::from("../svg"),
            outdir: PathBuf::from(&"Fuchsia-Pop"),
            colorscheme: ColorScheme::FuchsiaPop,
        },
        Build {
            name: "Fuchsia-Red".to_string(),
            comment: "Scalable Fuchsia Icons (Red)".to_string(),
            srcdir: PathBuf::from("../svg"),
            outdir: PathBuf::from(&"Fuchsia-Red"),
            colorscheme: ColorScheme::FuchsiaRed,
        },
    ];

    for build in builds {
        build.build().unwrap();
    }
}
