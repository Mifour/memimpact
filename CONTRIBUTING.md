# Contributing to memimpact

Thanks for your interest in contributing!
Memimpact is a learning-driven, performance-aware project, and contributions are welcome.
You don’t have to submit complex features to help.

This guide exists to make collaboration smooth, predictable, and enjoyable for everyone.

## Project Philosophy

memimpact is built with a few core principles:

- User's quality of life improvement
This project comes from the desired to have handy tool that measure memory consumption.

- Linux memory momnitoring
This is a Linux memory monitoring tool and Memimpact relies on proc fs. Support for other OSs is not planned because their memory management is different.

- Learning-oriented development
This project is as much about understanding how things work as it is about building something useful.

- Simplicity over cleverness
Clear, readable code is preferred over overly abstract or “smart” solutions.

- Low dependency surface
 We intentionally avoid adding external crates unless they provide major, clear value.  
 Reasons:

  - Better understanding of the internals

  - Lower long-term maintenance burden

  - Reduced security and supply-chain risk

  - More control over performance and behavior

If you keep these in mind, you’re already aligned with the project.

## Ways You Can Contribute

Valuable contributions include:

- Fixing bugs

- Improving documentation

- Refactoring for clarity

- Adding tests (both unittests and real world usecases)

- Improving performance

- Reporting issues or unclear behavior

- Suggesting better APIs or ergonomics

## Before You Start

If your change is more than a small fix:

- Open an issue first

- Describe the problem

- Explain your proposed solution

- Wait for alignment before large changes

This prevents wasted effort for all and keeps the direction consistent.

## Development Guidelines
### Code Style

Prefer explicit and readable code

Avoid unnecessary abstraction layers

Use descriptive names over short ones

Keep functions focused and small

If a future contributor can understand it quickly, it's good code.

### Dependencies (Important)

Please do not introduce new external crates without discussion.

You may propose a new dependency only if:

- It solves a complex problem that would be unreasonable to implement ourselves

- It is widely used and well maintained

- It significantly improves safety, correctness, or performance

If suggesting one:

- Justify why a custom implementation is not appropriate

- Explain trade-offs (size, maintenance, risk)

- PRs that add crates without discussion may be declined.

### Performance Awareness

Although performance is not the most critical aspect, since memimpact touches memory-related behavior:

- Avoid unnecessary allocations

- Be mindful of copies vs references

Document trade-offs when choosing clarity over raw performance

### Tests

Whenever possible:

- Add tests for new features

- Add regression tests for bug fixes

Prefer small, focused tests

## Pull Request Process

Fork the repo

Create a feature branch
feature/short-description or fix/short-description

Make your changes

Run tests and ensure everything builds

Open a Pull Request with:

- What changed

- Why it changed

- Any issue relevant to your PR 

- Any trade-offs or limitations

Small PRs get reviewed faster than large ones.

## Communication

Be respectful and constructive.
Questions, doubts, and alternative ideas are always welcome — discussion improves the project.
We all got job and life beyond this project.

## Final Note

memimpact is evolving, and so will this guide.
If something here is unclear, that’s a contribution opportunity too — open an issue or PR to improve this file.

Thanks for helping make memimpact better
