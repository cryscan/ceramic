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