# Mixin Rust

A Minecraft Modding style MIXIN manager for any open source Rust Program!

You can make mods for any Rust program that you have the source code of.
The mod defines little files, called 'mixins'.

Mixins is a word that comes from minecraft modding, appearing in the stage where Minecraft mods inject their code into specific locations of minecraft's source code to mod it.
The Minecraft modding ecosystem DIRECTLY inspired this project.

The code in the mixin files will be injected into the source code or even replacing some source code.
mixin_rust then compiles the program with the changes and runs it, creating a seamless modding experience akin to minecraft modding.

## Security
Because this project handles raw Rust source code, the mods potentially have access to any files that the user can access. 
The files could be Discord/Browser tokens, github credentials, cookies and pictures of your feet.
The security of mods made by strangers can't be guaranteed. That means that I have to try my best to sandbox the cargo build process and the application runtime process.
mixin_rust will allow the creator of the source code that is being modded to define a file in the source code root that gives explicit paths that are allowed through the sandbox.

More security may follow soon  **Trademark symbol here**

## Code Merging
The modder defines files like 'dogwhistle.rs.mixin' or just 'dogwhistle.mixin'. 
In those files it looks something like this, conceptually:

file.mixin
``` 
# Mixin Rust source-level mixin prototype

[target]
path = "crate::simulation::traffic::TrafficSystem::update"

[operation]
kind = "before"

[code]
source = """
my_custom_traffic_logic();
"""

```

Mixins will come in layers:
### Level 1: inject
```
before function
after function
around function
```
### Level 2: modify
```
replace expression
replace match arm
add struct field
add impl
add trait implementation
```
### Level 3: replace
```
replace function
replace type
replace module
replace file
```
### Level 4: chaos
```
arbitrary patch against generated source tree maybe
```

All of this is still just hypothetical though, we will see when I get there, it is a pretty long way...
