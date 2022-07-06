extern crate num_traits;

use nalgebra::{DMatrix, Dim, Matrix3, VecStorage, Vector2, Vector3, QR};

#[derive(Debug)]
/// Describes the direction in which the transform is performed
pub enum Direction {
    /// Slide is "fixed" and the transform describes transformation from another space to the slide space
    ToSlide,
    /// Slide is "moving" and the transform describes transformation from slide space to another space
    FromSlide,
}

/// Create a trait which captures necessary traits for matrix multiplication
pub trait TransformScalar:
    nalgebra::Scalar
    + num_traits::identities::Zero
    + num_traits::identities::One
    + nalgebra::ClosedAdd
    + nalgebra::ClosedMul
    + nalgebra::ComplexField
    + Copy
{
}
impl<T> TransformScalar for T where
    T: nalgebra::Scalar
        + num_traits::identities::Zero
        + num_traits::identities::One
        + nalgebra::ClosedAdd
        + nalgebra::ClosedMul
        + nalgebra::ComplexField
        + Copy
{
}

/// AffineTransform describes a mapping from one space to another while preserving parallel lines.
#[derive(Debug)]
pub struct AffineTransform<T>
where
    T: TransformScalar,
{
    direction: Direction,

    matrix: Matrix3<T>,
    inv_matix: Option<Matrix3<T>>,
}

fn points_to_dmatrix<T>(points: Vec<Vector2<T>>) -> DMatrix<T>
where
    T: TransformScalar,
{
    let mut data: Vec<T> = Vec::with_capacity(points.len() * 3);

    for coord in 0..3 {
        if coord < 2 {
            for index in 0..points.len() {
                let point = points.get(index).expect("Point should be present");

                data.push(point[coord]);
            }
        } else {
            for _index in 0..points.len() {
                data.push(T::one());
            }
        }
    }

    let vec_storage = VecStorage::new(Dim::from_usize(points.len()), Dim::from_usize(3), data);
    DMatrix::from_data(vec_storage)
}

impl<T> AffineTransform<T>
where
    T: TransformScalar,
{
    /// Returns the identity matrix as the transform
    pub fn identity() -> Self {
        let mut matrix = Matrix3::zeros();
        matrix.m11 = T::one();
        matrix.m22 = T::one();
        matrix.m33 = T::one();

        let inv_matrix = matrix.try_inverse();

        AffineTransform {
            direction: Direction::ToSlide,
            matrix,
            inv_matix: inv_matrix,
        }
    }

    /// Generate a transformation by solving the set of linear equations Ax = y
    pub fn from_points(moving_points: Vec<Vector2<T>>, fixed_points: Vec<Vector2<T>>) -> Self {
        let moving = points_to_dmatrix(moving_points);
        let fixed = points_to_dmatrix(fixed_points);

        let qr = QR::new(fixed);
        let res = qr.solve(&moving).unwrap();
        // Probably a better way to do this
        // Copy data from the solution to linear equations into Matrix4
        let mut matrix = Matrix3::zeros();

        matrix.m11 = res[(0, 0)];
        matrix.m21 = res[(0, 1)]; //*res.get((0, 1)).unwrap();
        matrix.m31 = res[(0, 2)]; //*res.get((0, 2)).unwrap();
        matrix.m12 = res[(1, 0)]; //*res.get((1, 0)).unwrap();
        matrix.m22 = res[(1, 1)]; //*res.get((1, 1)).unwrap();
        matrix.m32 = res[(1, 2)]; //*res.get((1, 2)).unwrap();
        matrix.m13 = res[(2, 0)]; //*res.get((2, 0)).unwrap();
        matrix.m23 = res[(2, 1)]; //*res.get((2, 1)).unwrap();
        matrix.m33 = T::one();

        AffineTransform {
            direction: Direction::ToSlide,
            matrix,
            inv_matix: matrix.try_inverse(),
        }
    }

    /// Creates an `AffineTransform` with the
    pub fn inverse_transform(&self) -> Option<AffineTransform<T>> {
        // TODO: invert the matrix if necessary
        let direction = match self.direction {
            Direction::ToSlide => Direction::FromSlide,
            Direction::FromSlide => Direction::ToSlide,
        };

        Some(AffineTransform {
            direction,
            matrix: Matrix3::<T>::identity() * self.inv_matix?,
            inv_matix: Some(Matrix3::<T>::identity() * self.matrix),
        })
    }

    /// Returns the transformation matrix in the "towards the slide" direction (from other space, to slide space)
    pub fn to_slide_matrix(&self) -> Option<&Matrix3<T>> {
        match self.direction {
            Direction::ToSlide => Some(&self.matrix),
            Direction::FromSlide => self.inv_matix.as_ref(),
        }
    }

    /// Returns the transformation matrix in the "away the slide" direction (from slide space, to other space)
    pub fn from_slide_matrix(&self) -> Option<&Matrix3<T>> {
        match self.direction {
            Direction::ToSlide => self.inv_matix.as_ref(),
            Direction::FromSlide => Some(&self.matrix),
        }
    }

    /*pub fn from_slide_matrix(&self) -> &Matrix3<T> {
        match self.direction {
            Direction::ToSlide => self.get_inv_matrix(),
            //   Direction::FromSlide => &self.matrix,
        }
    }*/

    /// Transforms a point in the external space to the slide space
    pub fn transform_to_slide(&self, x: T, y: T) -> Option<Vector3<T>> {
        let point = Vector3::new(x, y, T::one());
        let point = self.to_slide_matrix()? * point;

        Some(point)
    }

    /// Transforms a point in slide space to the external space
    pub fn transform_from_slide(&self, x: T, y: T) -> Option<Vector3<T>> {
        let point = Vector3::new(x, y, T::one());
        let point = self.from_slide_matrix()? * point;

        Some(point)
    }

    //pub fn transform_point(&self,
}
