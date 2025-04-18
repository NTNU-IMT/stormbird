pub fn second_order_smoothing<T>(x: &[T], smoothing_factor: f64) -> Vec<T>
where T: SmoothingOps
{
    let mut x_smooth: Vec<T> = Vec::with_capacity(x.len());

    x_smooth.push(x[0]);

    for i in 1..x.len()-1 {
        x_smooth.push(
            x[i] + (x[i-1] - x[i] * 2.0 + x[i+1]) * smoothing_factor
        );
    }

    x_smooth.push(*x.last().unwrap());

    x_smooth
}
