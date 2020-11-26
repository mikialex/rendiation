use crate::*;

macro_rules! into_mint_column_matrix {
    ($mint_name:ident, $rows:expr, $cols:expr $( , ($col_name:ident, $col_idx:expr ) )+) => {
        #[cfg(feature = "mint")]
        impl<T: Copy> Into<mint::$mint_name<T>> for Matrix<T, {$rows}, {$cols}> {
            fn into(self) -> mint::$mint_name<T> {
                mint::$mint_name {
                    $(
                        $col_name: self.0[$col_idx].into(),
                    )*
                }
            }
        }
    }
}

into_mint_column_matrix!(ColumnMatrix2, 2, 2, (x, 0), (y, 1));
into_mint_column_matrix!(ColumnMatrix3, 3, 3, (x, 0), (y, 1), (z, 2));
into_mint_column_matrix!(ColumnMatrix4, 4, 4, (x, 0), (y, 1), (z, 2), (w, 3));
into_mint_column_matrix!(ColumnMatrix2x3, 2, 3, (x, 0), (y, 1), (z, 2));
into_mint_column_matrix!(ColumnMatrix2x4, 2, 4, (x, 0), (y, 1), (z, 2), (w, 3));
into_mint_column_matrix!(ColumnMatrix3x2, 3, 2, (x, 0), (y, 1));
into_mint_column_matrix!(ColumnMatrix3x4, 3, 4, (x, 0), (y, 1), (z, 2), (w, 3));
into_mint_column_matrix!(ColumnMatrix4x2, 4, 2, (x, 0), (y, 1));
into_mint_column_matrix!(ColumnMatrix4x3, 4, 3, (x, 0), (y, 1), (z, 2));

macro_rules! from_mint_column_matrix {
    ($mint_name:ident, $rows:expr, $cols:expr, $($component:ident),+) => {
        #[cfg(feature = "mint")]
        impl<T> From<mint::$mint_name<T>> for Matrix<T, {$rows}, {$cols}> {
            fn from(m: mint::$mint_name<T>) -> Self {
                Self([
                    $(
                        Vector::<T, {$rows}>::from(m.$component),
                    )*
                ])
            }
        }
    }
}

from_mint_column_matrix!(ColumnMatrix2, 2, 2, x, y);
from_mint_column_matrix!(ColumnMatrix3, 3, 3, x, y, z);
from_mint_column_matrix!(ColumnMatrix4, 4, 4, x, y, z, w);
from_mint_column_matrix!(ColumnMatrix2x3, 2, 3, x, y, z);
from_mint_column_matrix!(ColumnMatrix2x4, 2, 4, x, y, z, w);
from_mint_column_matrix!(ColumnMatrix3x2, 3, 2, x, y);
from_mint_column_matrix!(ColumnMatrix3x4, 3, 4, x, y, z, w);
from_mint_column_matrix!(ColumnMatrix4x2, 4, 2, x, y);
from_mint_column_matrix!(ColumnMatrix4x3, 4, 3, x, y, z);

macro_rules! into_mint_row_matrix {
    ($mint_name:ident, $rows:expr, $cols:expr $( , ($col_name:ident, $col_idx:expr ) )+) => {
        #[cfg(feature = "mint")]
        impl<T: Copy> Into<mint::$mint_name<T>> for Matrix<T, {$rows}, {$cols}> {
            fn into(self) -> mint::$mint_name<T> {
                let transposed = self.transpose();
                mint::$mint_name {
                    $(
                        $col_name: transposed.0[$col_idx].into(),
                    )*
                }
            }
        }
    }
}

into_mint_row_matrix!(RowMatrix2, 2, 2, (x, 0), (y, 1));
into_mint_row_matrix!(RowMatrix3, 3, 3, (x, 0), (y, 1), (z, 2));
into_mint_row_matrix!(RowMatrix4, 4, 4, (x, 0), (y, 1), (z, 2), (w, 3));
into_mint_row_matrix!(RowMatrix2x3, 2, 3, (x, 0), (y, 1));
into_mint_row_matrix!(RowMatrix2x4, 2, 4, (x, 0), (y, 1));
into_mint_row_matrix!(RowMatrix3x2, 3, 2, (x, 0), (y, 1), (z, 2));
into_mint_row_matrix!(RowMatrix3x4, 3, 4, (x, 0), (y, 1), (z, 2));
into_mint_row_matrix!(RowMatrix4x2, 4, 2, (x, 0), (y, 1), (z, 2), (w, 3));
into_mint_row_matrix!(RowMatrix4x3, 4, 3, (x, 0), (y, 1), (z, 2), (w, 3));

// It would be possible to implement this without a runtime transpose() by
// directly copying the corresponding elements from the mint matrix to the
// appropriate position in the aljabar matrix, but it would be substantially
// more code to do so. I'm leaving it as a transpose for now in the expectation
// that converting between aljabar and mint entities will occur infrequently at
// program boundaries.
macro_rules! from_mint_row_matrix {
    ($mint_name:ident, $rows:expr, $cols:expr, $($component:ident),+) => {
        #[cfg(feature = "mint")]
        impl<T> From<mint::$mint_name<T>> for Matrix<T, {$rows}, {$cols}> {
            fn from(m: mint::$mint_name<T>) -> Self {
                Matrix::<T, {$cols}, {$rows}>([
                    $(
                        Vector::<T, {$cols}>::from(m.$component),
                    )*
                ]).transpose()
            }
        }
    }
}

from_mint_row_matrix!(RowMatrix2, 2, 2, x, y);
from_mint_row_matrix!(RowMatrix3, 3, 3, x, y, z);
from_mint_row_matrix!(RowMatrix4, 4, 4, x, y, z, w);
from_mint_row_matrix!(RowMatrix2x3, 2, 3, x, y);
from_mint_row_matrix!(RowMatrix2x4, 2, 4, x, y);
from_mint_row_matrix!(RowMatrix3x2, 3, 2, x, y, z);
from_mint_row_matrix!(RowMatrix3x4, 3, 4, x, y, z);
from_mint_row_matrix!(RowMatrix4x2, 4, 2, x, y, z, w);
from_mint_row_matrix!(RowMatrix4x3, 4, 3, x, y, z, w);
