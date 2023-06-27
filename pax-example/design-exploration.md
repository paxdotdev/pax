<Main>
    <SomeUnknownThing>
        ^ This tag-name will be lower-cased
        when parsed by browser
    </SomeUnknownThing>
    <AnotherThing>
        Should this just be XML?

        - Pro: parsing is easier. it's probably just a better tool.
        - Con: XML's brand sucks; html is universal
               and a better fit for mass appeal

        We totally lose the "fork the web" angle
        if we go with xml.
    </AnotherThing>
</Main>


<html>
    <stacker>
        <repeat data-list={{
            || -> Vec<Rc<PropertiesCoproduct />
            //... no.
                }}>
            <rectangle fill={{}}>
        </repeat>
    </stacker>
</html>

//is there a better, shorter name for `component`?
//it's overloaded with a different concept from game engines
//(the C in ECS) AND it would be ergonomically
//beneficial to have a shorter name for use in expressions,
//instead of `component.some_property`
//   Module, widget, entity, movie clip,
//   component,
//   ...


Alphabet of injectables:

`datum`: used by `repeat` to expose iterated data to expressions
        - How to reach across nested repeat frames? for i; for j

        -
`this`: (or `self`, `me`, `component`, etc.) used as a reference to
        the containing component and its properties / children
`




#template!
<stacker>
    <repeat data-list={{this.panels}}>
        |datum| {
            <rectangle fill={{Color.hsla(datum.bg_fill)}} />
        }
    </repeat>
</stacker>

#template-src!("./path/to/src.html")

#template-behind! //automatically load same_file_name.html


#template!
<stacker>
    @foreach (panel in panels) {
        <rectangle fill={{Color.hsla(panel.bg_fill)}} />
    }
</stacker>





#template!(
<stacker id="outer-stacker">
    @foreach (panel in panels) {
        <rectangle fill={{Color.hsla(panel.bg_fill)}} />
    }
</stacker>
)





// Properties:  can be inlined or declared alongside
#properties!(
    #outer-stacker {
        size: (Size::Percent(100.0),Size::Percent(100.0))
        transform: || {}
        direction:
        cells:
        gutter:
    }
)
// We really want language server aid here...

// What about in rust, outside of macro:

properties: [ //Vec<PropertiesCoproduct>
    #join!(#outer-stacker) { //automatically determine that this is a StackerProperties

    }
]


**If we commit to full custom parsers**, we could do a CSS-like
    syntax, handling literals as needed w/ parser —
and another parser for expressions.

That is, three total parsers:
    - CSS-like parser for property-value binding
    - Expression parser, including stream injection
    - Template parser, Blazor-like


// Properties:  can be inlined or declared alongside
```

#template!(
    <Stacker id="outer-stacker">
        <Rectangle id="rect-0" />
        <Rectangle id="rect-1" />
        <Rectangle id="rect-2" />
    </Stacker>
)


#properties!(
    #outer-stacker {
        size: 50px, (dash) => {
            dash.height
        }px, //expression syntax is JS-lambda-like,
        transform: {
            translate: 20px, x, //x is "don't care" i.e. "default"
            scale: 100%, 90%,
            anchor: 50%, 50%,
        }
        orientation: vertical, //enums are tricky.  single global keywords are most ergonomic
                               //but have obvious namespace collision risks.
                               //To start we could give judicious global real estate to 
                               //orientation.{vertical|horizontal}, toward.{top,right,bottom,left}
        cells: 10,
        gutter: 10px,
    }
    
    #rect-0 {
        
    }
)








#properties!(
    #outer-stacker {
        size: (50px, (dash) => {
            dash.height
        }px), //expression syntax is JS-lambda-like,
        transform: {
            translate: (20px, x); //x is "don't care" i.e. "default"
            scale: (100%, 90%);
            anchor: (50%, 50%),
        }
        orientation: vertical, //enums are tricky.  single global keywords are most ergonomic
                               //but have obvious namespace collision risks.
                               //To start we could give judicious global real estate to 
                               //orientation.{vertical|horizontal}, toward.{top,right,bottom,left}
        cells: 10,
        gutter: 10px,
    }
    
    #rect-0 {
        
    }
)




```

It's important to have auto-complete here.  We'll want to parse
(or duplicate/declare, in a manifest) the source typedefs




```rust
#template!(
    <Stacker id="outer-stacker">
        <Rectangle id="rect-0" width=@{num_clicks * 20} />
        <Rectangle id="rect-1" />
        <Rectangle id="rect-2" />
    </Stacker>
)

#properties!(
    #outer-stacker {
        
    }
)

```




## Expressions — should be CEL-like

Snippet from: https://github.com/google/cel-spec/blob/7972b9076513e6a4bbd184f9d073db949ea53c65/README.md
```
// Condition
account.balance >= transaction.withdrawal
    || (account.overdraftProtection
    && account.overdraftLimit >= transaction.withdrawal  - account.balance)

// Object construction
common.GeoPoint{ latitude: 10.0, longitude: -5.5 }
```


