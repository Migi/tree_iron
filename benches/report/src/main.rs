use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

fn get_median(test_type: &str, tree_type: &str, algo: &str) -> f64 {
    let file = File::open(Path::new(&format!("../../target/criterion/{}_{}_{}/base/estimates.json", test_type, tree_type, algo)))
        .unwrap_or_else(|_| panic!("Couldn't open file \"../../target/criterion/{}_{}_{}/base/estimates.json\"", test_type, tree_type, algo));
    let reader = BufReader::new(file);
    let val : serde_json::Value = serde_json::de::from_reader(reader).unwrap();

    val["Median"]["point_estimate"].as_f64().unwrap()
}

fn plot_tree_type(test_type: &str, tree_type: &str, algos: &[&str], start_pos: f64, mut writer: impl Write) {
    let packed_median = get_median(test_type, tree_type, "packed");

    for (i,algo) in algos.iter().enumerate() {
        let algo_median = get_median(test_type, tree_type, algo);

        writeln!(writer, "{}\t{}\t{}\t0x{:02X}{:02X}{:02X}", start_pos+(i as f64)*0.2, algo, algo_median/packed_median, 41-i*4, 103-i*10, 204-i*20).unwrap();
    }
}

fn plot_test_type(test_type: &str, algos: &[&str], ymax:f64) {
    let tree_types = ["small", "shallow", "binary", "wide_random", "deep_random"];

    // write the data
    {
        let file = File::create(Path::new(&format!("./graphs/{}.txt", test_type))).unwrap();
        let mut writer = BufWriter::new(file);

        for (i,tree_type) in tree_types.iter().enumerate() {
            plot_tree_type(test_type, tree_type, &algos, 1. + (i as f64)*3., &mut writer);
        }
    }

    // plot the graphs
    {
        let gp_text = format!("
set term png noenhanced
set output \"./graphs/{test_type}.png\"
set boxwidth 0.1
set yrange [0 : {ymax}]
set style fill solid 0.5

set arrow from -0.5,1 to 8,1 nohead

plot \"./graphs/{test_type}.txt\" using 1:3:4 with boxes lc rgb var notitle, \"\"  using ($1):($3+0.2):(sprintf(\"%3.2f\",$3)) with labels font \"Arial,8\" rotate by 90 notitle

set output
", test_type=test_type, ymax=ymax);
        
        let file = File::create(Path::new(&format!("./graphs/{}.gp", test_type))).unwrap();
        let mut writer = BufWriter::new(file);
        write!(writer, "{}", gp_text).unwrap();
        writer.flush().unwrap();
    }
    
    println!("Plotting: gnuplot ./graphs/{}.gp", test_type);

    let output = std::process::Command::new("gnuplot")
        .arg(format!("./graphs/{}.gp", test_type))
        .output()
        .unwrap_or_else(|_| panic!(format!("failed to execute \"gnuplot ./graphs/{}.gp\"", test_type)));
    
    let stdout = std::str::from_utf8(&output.stdout).unwrap();
    let stderr = std::str::from_utf8(&output.stderr).unwrap();
    if stdout != "" {
        println!("Stdout: {}", stdout);
    }
    if stderr != "" {
        println!("Stderr: {}", stderr);
    }
    println!("Done.");
}

fn main() {
    std::fs::create_dir_all("./graphs").unwrap();

    let algos = ["packed", "es", "bump", "ego", "index", "vec", "naive", "ll", "id"];

    plot_test_type("make", &algos, 7.);
    plot_test_type("hash", &algos, 2.);
    plot_test_type("bfs", &algos, 2.);
}