use std::fmt;
use nalgebra::{DMatrix, VecStorage, Dim, Vector3, Matrix3};

enum Direction {
    ToMoving,
    ToFixed
}

pub struct AffineTransform<T: Copy + fmt::Debug + fmt::Display> {
    matrix: Option<Matrix3<T>>,
}

impl<T: Copy + fmt::Debug + fmt::Display> AffineTransform<T> {
    pub fn from_points(moving_points: Vec<Vector3<T>>, fixed_points: Vec<Vector3<T>>) -> Self {
        let mut data: Vec<T> = Vec::with_capacity(moving_points.len()*3);

        for index in 0..moving_points.len() {
            let point = moving_points.get(index).expect("Point should be present");

            data.push(point[0]);
            data.push(point[1]);
            data.push(point[2]);
        }

        let vec_storage = VecStorage::new(Dim::from_usize(moving_points.len()), Dim::from_usize(3), data);
        let moving = DMatrix::from_data(vec_storage);

        println!("{:?}", moving);

        AffineTransform {
            //matrix: moving,
            matrix: None
        }
    }
}