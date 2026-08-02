#[allow(unused)]
use heap_tools::PreciseAllocationStatistics;

#[cfg(feature = "mimalloc")]
#[allow(unused)]
type BaseAllocator = mimalloc::MiMalloc;
#[cfg(feature = "mimalloc")]
#[allow(unused)]
const BASE_ALLOCATOR: BaseAllocator = mimalloc::MiMalloc;

#[cfg(not(feature = "mimalloc"))]
#[allow(unused)]
type BaseAllocator = std::alloc::System;
#[cfg(not(feature = "mimalloc"))]
#[allow(unused)]
const BASE_ALLOCATOR: BaseAllocator = std::alloc::System;

// global_allocator priority: dhat-heap-profiling > tracy-heap-debug > base
#[cfg(feature = "dhat-heap-profiling")]
#[global_allocator]
pub static GLOBAL_ALLOCATOR: PreciseAllocationStatistics<dhat::Alloc> =
  PreciseAllocationStatistics::new(dhat::Alloc);

#[cfg(all(not(feature = "dhat-heap-profiling"), feature = "tracy-heap-debug"))]
#[global_allocator]
pub static GLOBAL_ALLOCATOR: PreciseAllocationStatistics<
  tracing_tracy::client::ProfiledAllocator<BaseAllocator>,
> = PreciseAllocationStatistics::new(tracing_tracy::client::ProfiledAllocator::new(
  BASE_ALLOCATOR,
  64,
));

#[cfg(all(
  not(feature = "dhat-heap-profiling"),
  not(feature = "tracy-heap-debug")
))]
#[global_allocator]
pub static GLOBAL_ALLOCATOR: PreciseAllocationStatistics<BaseAllocator> =
  PreciseAllocationStatistics::new(BASE_ALLOCATOR);
