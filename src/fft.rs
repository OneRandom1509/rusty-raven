use num_complex::Complex32;
use std::f32::consts::PI;

fn dft(input: &[f32], output: &mut [Complex32], n: usize) {
    for f in 0..n {
        output[f] = Complex32::new(0.0, 0.0);
        for i in 0..n {
            let t = i as f32 / n as f32;
            let exp = Complex32::new(0.0, 2.0 * PI * f as f32 * t).exp();
            output[f] += input[i] * exp;
        }
    }
}

fn fft(input: &[f32], stride: usize, output: &mut [Complex32], n: usize) {
    assert!(n > 0);

    if n == 1 {
        output[0] = Complex32::new(input[0], 0.0);
        return;
    }

    fft(input, stride * 2, &mut output[..n / 2], n / 2);
    fft(&input[stride..], stride * 2, &mut output[n / 2..], n / 2);

    for k in 0..n / 2 {
        let t = k as f32 / n as f32;
        let v = Complex32::new(0.0, -2.0 * PI * t).exp() * output[k + n / 2];
        let e = output[k];
        output[k] = e + v;
        output[k + n / 2] = e - v;
    }
}

fn main() {
    let n = 4096;
    let mut input = vec![0.0; n];
    let mut output = vec![Complex32::new(0.0, 0.0); n];

    for i in 0..n {
        let t = i as f32 / n as f32;
        input[i] = (2.0 * PI * t).cos() + (2.0 * PI * t * 2.0).sin();
    }

    fft(&input, 1, &mut output, n);

    for f in 0..n {
        println!("{}: {:.2}, {:.2}", f, output[f].re, output[f].im);
    }

