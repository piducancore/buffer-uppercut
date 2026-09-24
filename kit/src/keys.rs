//! Stable product key identifiers; independent of platform key codes.

use buffer_uppercut_dsp::NUM_PADS;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KeyId(u8);

impl KeyId {
    pub const UNASSIGNED: Self = Self(0);

    #[must_use]
    pub const fn from_id(id: u8) -> Option<Self> {
        if id <= 47 { Some(Self(id)) } else { None }
    }

    #[must_use]
    pub const fn id(self) -> u8 {
        self.0
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        const LABELS: [&str; 48] = [
            "—", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P",
            "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z", "0", "1", "2", "3", "4", "5", "6",
            "7", "8", "9", "-", "=", "[", "]", "\\", ";", "'", "`", ",", ".", "/",
        ];
        LABELS[self.0 as usize]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyMapError {
    UnsupportedKey(u8),
    Conflict { pad: usize },
    InvalidPad,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyMap([KeyId; NUM_PADS]);

impl Default for KeyMap {
    fn default() -> Self {
        Self([28, 29, 30, 31, 17, 23, 5, 18, 1, 19, 4, 6, 26, 24, 3, 22].map(KeyId))
    }
}

impl KeyMap {
    #[must_use]
    pub fn ids(self) -> [u8; NUM_PADS] {
        self.0.map(KeyId::id)
    }

    /// # Errors
    /// Rejects unsupported identifiers and duplicate assigned keys.
    pub fn from_ids(ids: [u8; NUM_PADS]) -> Result<Self, KeyMapError> {
        let mut map = Self([KeyId::UNASSIGNED; NUM_PADS]);
        for (pad, id) in ids.into_iter().enumerate() {
            map.assign(
                pad,
                KeyId::from_id(id).ok_or(KeyMapError::UnsupportedKey(id))?,
            )?;
        }
        Ok(map)
    }

    #[must_use]
    pub fn key(self, pad: usize) -> Option<KeyId> {
        self.0.get(pad).copied()
    }

    #[must_use]
    pub fn pad_for(self, key: KeyId) -> Option<usize> {
        if key == KeyId::UNASSIGNED {
            return None;
        }
        self.0.iter().position(|candidate| *candidate == key)
    }

    /// # Errors
    /// Returns the occupied pad without changing either binding, or rejects an invalid pad.
    pub fn assign(&mut self, pad: usize, key: KeyId) -> Result<(), KeyMapError> {
        if pad >= NUM_PADS {
            return Err(KeyMapError::InvalidPad);
        }
        if let Some(other) = self.pad_for(key)
            && other != pad
        {
            return Err(KeyMapError::Conflict { pad: other });
        }
        self.0[pad] = key;
        Ok(())
    }

    /// # Errors
    /// Rejects invalid pad indices without modifying the mapping.
    pub fn swap(&mut self, a: usize, b: usize) -> Result<(), KeyMapError> {
        if a >= NUM_PADS || b >= NUM_PADS {
            return Err(KeyMapError::InvalidPad);
        }
        self.0.swap(a, b);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_physical_grid_and_conflicts_are_explicit() {
        let mut map = KeyMap::default();
        let labels: Vec<_> = (0..NUM_PADS)
            .map(|pad| map.key(pad).unwrap().label())
            .collect();
        assert_eq!(
            labels,
            [
                "1", "2", "3", "4", "Q", "W", "E", "R", "A", "S", "D", "F", "Z", "X", "C", "V"
            ]
        );
        let old = map;
        assert_eq!(
            map.assign(0, map.key(1).unwrap()),
            Err(KeyMapError::Conflict { pad: 1 })
        );
        assert_eq!(map, old);
        map.swap(0, 1).unwrap();
        assert_eq!(map.key(0), old.key(1));
        assert_eq!(map.key(1), old.key(0));
        map.assign(0, KeyId::UNASSIGNED).unwrap();
        map.assign(1, KeyId::UNASSIGNED).unwrap();
        assert_eq!(map.pad_for(KeyId::UNASSIGNED), None);
        assert_eq!(KeyMap::from_ids(map.ids()).unwrap(), map);
    }

    #[test]
    fn malformed_maps_are_rejected() {
        assert_eq!(
            KeyMap::from_ids([48; NUM_PADS]),
            Err(KeyMapError::UnsupportedKey(48))
        );
        assert_eq!(
            KeyMap::from_ids([1; NUM_PADS]),
            Err(KeyMapError::Conflict { pad: 0 })
        );
        assert!(KeyMap::from_ids([0; NUM_PADS]).is_ok());
        let mut map = KeyMap::default();
        let old = map;
        assert_eq!(map.swap(0, NUM_PADS), Err(KeyMapError::InvalidPad));
        assert_eq!(
            map.assign(NUM_PADS, KeyId::UNASSIGNED),
            Err(KeyMapError::InvalidPad)
        );
        assert_eq!(map, old);
    }
}
