
use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

/// Returns the point on a triangle, specified as an array of spatial vectors, that is closest to 
/// the supplied input point. 
/// 
/// Source: [https://www.geometrictools.com/Documentation/DistancePoint3Triangle3.pdf]
pub fn closest_point_on_triangle(
    point: SpatialVector, 
    triangle: [SpatialVector; 3]
) -> SpatialVector {
    let [a, b, c] = triangle;

    let ab = b - a;
    let ac = c - a;
    let ap = point - a;

    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);

    // Check if vertex A is closest
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }

    let bp = point - b;

    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    
    // Check if vertex B is closest
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }

    let cp = point - c;

    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);

    // Check if vertex C is closest
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    
    // Check for edge AB
    let vc = d1*d4 - d3*d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);

        return a + v * ab;
    }

    // Check for edge AC
    let vb = d5*d2 - d1*d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return a + w * ac;
    }
        
    // Check for edge BC
    let va = d3*d6 - d5*d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return b + w * (c - b);
    }
        
    // Inside the triangle
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;

    a + v * ab + w * ac
}

pub fn distance_to_triangle(point: SpatialVector, triangle: [SpatialVector; 3]) -> Float {
    let closest_point = closest_point_on_triangle(point, triangle);

    point.distance(closest_point)
}

pub fn solid_angle(point: SpatialVector, triangle: [SpatialVector; 3]) -> Float {
    let a = (triangle[0] - point).normalize();
    let b = (triangle[1] - point).normalize();
    let c = (triangle[2] - point).normalize();

    let numerator = a.dot(b.cross(c));
    let denominator = 1.0 + a.dot(b) + b.dot(c) + a.dot(c);

    2.0 * numerator.atan2(denominator)
}
    