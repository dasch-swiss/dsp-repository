# DSP-Repository

This is the monorepo for the DSP Repository,
containing all the code and documentation for the DSP Archive and DPE.

For more information about, see the [documentation](docs/src/introduction.md).

You may also view the rendered documentation by following these steps:
- Verify you have just and cargo installed
- Install all dependencies with `just install-requirements`
- Build and run the documentation with `just docs-serve`
- Open your browser and navigate to `http://localhost:3000`

## License

This repository contains code under multiple licenses:
- Apache 2.0: All code except `modules/design_system/components/` directory
- All Rights Reserved: Code in `modules/design_system/components/` directory

See LICENSE-Apache-2.0 and LICENSE-AllRightsReserved for full terms.
