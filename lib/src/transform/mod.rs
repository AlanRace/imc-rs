extern crate num_traits;

use nalgebra::{DMatrix, Dim, Matrix3, VecStorage, Vector2, Vector3, QR};

#[derive(Debug)]
pub enum Direction {
    ToSlide,
    FromSlide,
}

// Create a trait which captures necessary traits for matrix multiplication
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

#[derive(Debug)]
pub struct AffineTransform<T>
where
    T: TransformScalar,
{
    direction: Direction,

    matrix: Matrix3<T>,
    inv_matix: Option<Matrix3<T>>,
}

fn to_dmatrix<T>(points: Vec<Vector2<T>>) -> DMatrix<T>
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

    //println!("{:?}", data);

    let vec_storage = VecStorage::new(Dim::from_usize(points.len()), Dim::from_usize(3), data);
    DMatrix::from_data(vec_storage)
}

impl<T> AffineTransform<T>
where
    T: TransformScalar,
{
    pub fn from_points(moving_points: Vec<Vector2<T>>, fixed_points: Vec<Vector2<T>>) -> Self {
        let moving = to_dmatrix(moving_points);
        let fixed = to_dmatrix(fixed_points);

        //println!("{:?}", moving);
        //println!("{:?}", fixed);
        //println!("{:?}", moving.dot(&fixed));

        let qr = QR::new(fixed); //.lu();
                                 //println!("{:?}", qr);
        let res = qr.solve(&moving).unwrap();
        // Probably a better way to do this
        // Copy data from the solution to linear equations into Matrix4
        let mut matrix = Matrix3::zeros();
        matrix.m11 = *res.get((0, 0)).unwrap();
        matrix.m21 = *res.get((0, 1)).unwrap();
        matrix.m23 = *res.get((0, 2)).unwrap();
        matrix.m12 = *res.get((1, 0)).unwrap();
        matrix.m22 = *res.get((1, 1)).unwrap();
        matrix.m32 = *res.get((1, 2)).unwrap();
        matrix.m13 = *res.get((2, 0)).unwrap();
        matrix.m23 = *res.get((2, 1)).unwrap();
        //matrix.m33 = *res.get((2, 2)).unwrap();
        //matrix.m44 = T::one();
        matrix.m33 = T::one();

        //println!("{:?}", matrix);

        AffineTransform {
            direction: Direction::ToSlide,
            matrix: matrix,
            inv_matix: None,
        }
    }

    fn get_inv_matrix(&self) -> &Matrix3<T> {
        // TODO: invert the matrix if necessary
        &self.matrix
    }

    pub fn to_slide_matrix(&self) -> &Matrix3<T> {
        match self.direction {
            Direction::ToSlide => &self.matrix,
            Direction::FromSlide => self.get_inv_matrix()
        }
    }

    pub fn from_slide_matrix(&self) -> &Matrix3<T> {
        match self.direction {
            Direction::ToSlide => self.get_inv_matrix(),
            Direction::FromSlide => &self.matrix
        }
    }
    

    pub fn transform_to_slide(&self, x: T, y: T) -> Option<Vector3<T>> {
        let point = Vector3::new(x, y, T::one());
        let point = self.to_slide_matrix() * point;

        Some(point)
    }

    //pub fn transform_point(&self, 
}
