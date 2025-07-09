+++
title = "Syntax"
weight = 5
+++

The Braisé syntax aims to be as simple and intuitive as possible. Given the fact that it still in very early development, it is subject to change, but the core concepts should remain the same.

## Recipes

The configuration structure mainly revolves around so-called "**recipes**", which are basically just tasks.

A recipe block looks like:

```braise
recipe "name" {
    // statements
}
```

Each statement within that recipe will be evaluated whenever the recipe is called: either from the CLI (`braise <name>`) or from another recipe:

```braise
recipe "number1" {
    print "Hello from number1!"
}

recipe "number2" {
    call @number1
}
```

## Parameters

You can define parameters for your recipes, which can be used to pass values when calling the recipe. Parameters are defined using the `param` keyword:

```braise
recipe "greet" {
    param name: string? = "World"
    print "Hello, ${name}!"
}
```

Parameters can be optional, either by providing a default value or by using the `?` suffix. When calling the recipe, you can override the default value:

```bash
braise greet --name "Chef"
```

But wait! We can also pass parameters when calling a recipe from another recipe:

```braise
recipe "greet" {
    param name: string? = "World"
    print "Hello, ${name}!"
}

recipe "main" {
    call @greet(name: "Chef")
}
```

## Types

As you may have noticed, you can specify types for parameters. Braisé supports several basic types:

<table>
<thead>
<tr>
<th>Type</th>
<th>Description</th>
<th>Example</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>string</code></td>
<td>A sequence of characters</td>
<td>

```braise
param name: string = "World"
```
</td>

</tr>
<tr>
<td><code>number</code></td>
<td>Numeric value</td>
<td>

```braise
param age: number = 30
param price: number = 19.99
```
</td>
</tr>
<tr>
<td><code>bool</code></td>
<td>true or false</td>
<td>

```braise
param active: bool = true
```
</td>
</tr>
<tr>
<td><code>[type]</code></td>
<td>List of values</td>
<td>

```braise
param items: [string] = ["apple", "banana", "cherry"]
```
</td>
</tr>
<tr>
<td><code>enum</code></td>
<td>Fixed set of values</td>
<td>

```braise
param color: ["red", "green", "blue"] = "red"
```
</td>
</tr>
</tbody>
</table>

## Variables

You can define variables using the `let` keyword. Variables can be used to store values that you want to reuse within a recipe:

```braise
recipe "calculate" {
    let a = 5
    let b = 10
    let sum = a + b
    print "The sum of ${a} and ${b} is ${sum}."
}
```

Variable types are inferred from the assigned value, but you can also specify a type explicitly:

```braise
recipe "calculate" {
    let a = "5"
    let b = "10"
    let sum: number = a + b 
    print "The sum of ${a} and ${b} is ${sum}."
}
```

Will result in:

```
The sum of 5 and 10 is 15.
```

That's because Braisé will automatically try and match the types of the variables. If they are not compatible, it will throw a type error.