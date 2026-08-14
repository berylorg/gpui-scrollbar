/// Application-neutral identity for the caller that owns one scroll model.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ScrollbarOwnerId(u64);

impl ScrollbarOwnerId {
    /// Creates an owner identity from a caller-controlled stable value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the caller-controlled identity value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Generation of one mounted scrollbar for an owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ScrollbarMountGeneration(u64);

impl ScrollbarMountGeneration {
    /// Creates a caller-controlled mount generation.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the generation value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Exact identity of one currently mounted scrollbar owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ScrollbarOwnerKey {
    /// Caller-controlled owner identity.
    pub owner_id: ScrollbarOwnerId,
    /// Caller-controlled generation of this mount.
    pub mount_generation: ScrollbarMountGeneration,
}

impl ScrollbarOwnerKey {
    /// Creates an exact owner-and-mount key.
    #[must_use]
    pub const fn new(
        owner_id: ScrollbarOwnerId,
        mount_generation: ScrollbarMountGeneration,
    ) -> Self {
        Self {
            owner_id,
            mount_generation,
        }
    }
}

/// Monotonic sequence for one managed-visibility activity lifecycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct ScrollbarVisibilitySequence(u64);

impl ScrollbarVisibilitySequence {
    pub(crate) const INITIAL: Self = Self(0);

    pub(crate) fn next(self) -> Self {
        Self(
            self.0
                .checked_add(1)
                .expect("scrollbar visibility sequence exhausted"),
        )
    }

    /// Returns the sequence value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Exact key retained by one managed delay, fade, or animation-frame driver.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ScrollbarVisibilityKey {
    /// Exact mounted owner identity.
    pub owner: ScrollbarOwnerKey,
    /// Monotonic visibility activity sequence.
    pub sequence: ScrollbarVisibilitySequence,
}
