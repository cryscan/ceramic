use amethyst::core::math::{Dynamic, MatrixMN, RealField, U1, U3, UnitQuaternion, Vector3};

pub mod transform;

/// Calculate the optimal translation and rotation that minimizes distance between two point sets.
pub fn match_shape<T: RealField>(origins: Vec<T>, targets: Vec<T>, eps: T, max_iter: usize) -> (Vector3<T>, UnitQuaternion<T>) {
    let ref origins = MatrixMN::<T, U3, Dynamic>::from_vec(origins);
    let ref targets = MatrixMN::<T, U3, Dynamic>::from_vec(targets);

    let origins_mean = origins.column_mean();
    let targets_mean = targets.column_mean();
    let translation = targets_mean - origins_mean;

    let origins = origins - origins_mean * MatrixMN::<T, U1, Dynamic>::repeat(origins.ncols(), T::one());
    let targets = targets - targets_mean * MatrixMN::<T, U1, Dynamic>::repeat(targets.ncols(), T::one());
    let ref covariance = origins * targets.transpose();
    let rotation = UnitQuaternion::from_matrix_eps(covariance, eps, max_iter, UnitQuaternion::identity());

    (translation, rotation)
}

/*
/// Verlet integration.
pub fn verlet<T: RealField, F>(
    position: Point3<T>,
    velocity: Vector3<T>,
    field: F,
    delta_seconds: T,
) -> (Point3<T>, Vector3<T>)
    where F: Fn(&Point3<T>) -> Vector3<T> {
    let half = T::from_str_radix("0.5", 10).ok().expect("Unreachable: convert from 0.5");
    let acceleration = field(&position);
    let velocity = velocity + acceleration.scale(delta_seconds * half);
    let position = position + velocity.scale(delta_seconds);

    let acceleration = field(&position);
    let velocity = velocity + acceleration.scale(delta_seconds * half);
    (position, velocity)
}
 */