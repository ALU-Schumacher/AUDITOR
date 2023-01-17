<p align="center">
  <a href="https://crates.io/crates/auditor"
    ><img
      src="https://img.shields.io/crates/v/auditor?style=flat-square"
      alt="Crates.io version"
  /></a>
  <a href="https://pypi.org/project/python-auditor/">
    <img alt="PyPI" src="https://img.shields.io/pypi/v/python-auditor?label=pyauditor&style=flat-square">
  </a>
  <a href="https://github.com/alu-schumacher/AUDITOR/actions"
    ><img
      src="https://img.shields.io/github/workflow/status/alu-schumacher/AUDITOR/Auditor/main?label=Auditor CI&style=flat-square"
      alt="GitHub Actions workflow status"
  /></a>
  <a href="https://github.com/orgs/ALU-Schumacher/packages"
    ><img
      src="https://img.shields.io/github/workflow/status/alu-schumacher/AUDITOR/Containers/main?label=Docker builds&style=flat-square"
      alt="GitHub Actions workflow status"
  /></a>
  <a href="https://github.com/alu-schumacher/AUDITOR/actions"
    ><img
      src="https://img.shields.io/github/workflow/status/alu-schumacher/AUDITOR/RPM/main?label=RPM builds&style=flat-square"
      alt="GitHub Actions workflow status"
  /></a>
  <img
    src="https://img.shields.io/crates/l/auditor?style=flat-square"
    alt="License"
  />
</p>

# AUDITOR

**AUDITOR** is short for 
<b>A</b>cco<b>U</b>nting <b>D</b>ata handl<b>I</b>ng <b>T</b>oolbox for <b>O</b>pportunistic <b>R</b>esources.
It allows one to flexibly build accounting pipelines for various use cases and environments.
AUDITOR sits at the core of the pipeline as the provider of the storage for the accounting records.
Via a REST interface, records can be pushed into or pulled from AUDITOR.
*Collectors* are used to collect accounting data form a source and push it to AUDITOR, while *plugins* pull data from AUDITOR for further processing.
Plugins and collectors are problem- and environment-specific and can be combined as needed. 
A Python library handles the interaction with AUDITOR and as such enables quick and easy development of collectors and plugins.

For more information on how to obtain and use AUDITOR, please [visit our website](https://alu-schumacher.github.io/AUDITOR/).


<p align="center">
  <a href="https://alu-schumacher.github.io/AUDITOR/">Documentation</a>
  |
  <a href="https://docs.rs/auditor">API Documentation</a>
  |
  <a href="https://alu-schumacher.github.io/AUDITOR/pyauditor/">pyauditor Documentation</a>
</p>


<p align="center">
  <img
    width="700"
    src="https://raw.githubusercontent.com/alu-schumacher/AUDITOR/main/media/auditor_overview.png"
  />
</p>



## License

Licensed under either of

 - Apache License, Version 2.0, ([LICENSE-APACHE](https://github.com/ALU-Schumacher/AUDITOR/blob/main/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 - MIT License ([LICENSE-MIT](https://github.com/ALU-Schumacher/AUDITOR/blob/main/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
