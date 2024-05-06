# Changelog

## desec_api 0.3.2 (2024-05-06)

### Added

### Changed

### Fixed

- Lowered the mistakenly published MSRV of 1.77.2 down to 1.63.0 (lowering MSRV should not be a breaking change)


## desec_api 0.3.1 (2024-05-06)

### Added

### Changed

- Comment to create_rrset & patch_rrset to wrap records for TXT rrsets in double quotes

### Fixed

- Creation of rrsets at domain apex failed becaus of the use of @ instead of an empty string for subname


## desec_api 0.3.0 (2024-05-04)

### Added

- A lot of issing API endpoints
- Documentation

### Changed

- The way the internal HTTP client is used
- Centralized error handling

### Fixed


## desec_api 0.2.0 (2024-04-25)

### Added

### Changed

- Replace String with &str in some places
- Overall cleanup

### Fixed

- Make some field in domain struct optional

## desec_api 0.1.0 (2024-04-25)

### Added

- Initial version

### Changed

### Fixed
