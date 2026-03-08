---
name: bevy-dev
description: Check the documentation of bevy and follow when develop game with bevy.
---
# How to use bevy 0.18
Follow the official documentation of bevy, and check the examples in the bevy repository.
https://docs.rs/bevy/latest/bevy/index.html

# Build UI with bevy_ui
you can check the documentation of bevy_ui, and follow the examples in the bevy repository.
https://docs.rs/bevy_ui/latest/bevy_ui/


# reference the cheat book
You can check the bevy cheat book for more examples and best practices when developing with bevy. The cheat book is a community-driven resource that provides tips, tricks, and examples for using bevy effectively.

But code in this book is not always up to date, so you should check the bevy version of each page to see if it is compatible with the version of bevy you are using. Though the code may not be up to date, the concepts and best practices are still valuable and can be applied to your development with bevy. Especially
the [chapter 14](https://bevy-cheatbook.github.io/programming.html), which introduces the basic mindset of bevy ecs programming.

https://bevy-cheatbook.github.io/

## Example of bevy_ui
You can check the examples in the bevy repository, such as:

https://github.com/bevyengine/bevy/tree/latest/examples/ui

## Use `children![]` macro to create UI hierarchy
You should use the `children![]` macro to create a hierarchy of UI elements, instead of `with_children` for better readability and maintainability.

## Extract the common UI code into a separate function
You should extract the common UI code into a separate function, to avoid code duplication and improve readability.

## Don't use magic values
Key values such as line heights, font sizes, and colors should be defined as constants or configuration variables, rather than hardcoded in the code. This makes it easier to maintain and update the UI design in the future. And you can also use colors in `bevy::color::palettes` instead of hardcoding color values.

# Organize your code
Don't shitting everywhere, you are supposed to organize your code level by level, module by module. You should collect related e/c/s into one plugin, and collect 
systems with similar functionality into one `SystemSet`. This makes it easier to understand the structure of your code and maintain it in the future.
