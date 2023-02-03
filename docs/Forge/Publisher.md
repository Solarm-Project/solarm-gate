## Assumptions
- Users do not want to be responsible for things they do in their free time
- Users want to be able to adjust packages and not need to join social circles
- Maintainers want to know what changes that are done to the packages they maintain under whatever publisher they might be.
- There must be some sort of Social interaction between the users of the packages and the publishers.
- Generic mechanisms lead to undefined behaviour
- Packaging instructions and metadata is not Source code and therefore not worthy of copyright protection
## Decisions
- Each user ([Idendity](Idendity)) will be given a personal publisher starting with the Tilde (~) character and their username called home publisher
- The Personal publisher is only accessible for that user and nobody else. Definitions can be shared however
- Public packages must be under a non home publisher.
- All public publishers can be overtaken with a to be defined process (for now Instance admin intervation)
- All public publishers have a Group that manages it consisting of at least two members or the one user maintaining it must be admin of the Instance
- Each Publisher has a so called gate, defining which packages are in which versions and adjustments to the package definitions.
- A publishers name refers to a group of people not a thing.
