
### 2024-08-09 - Initializing pax-designer

**Broad flow**

if we’re in main and designtime
    (AND this is not PaxDesigner itself — note that this code will be included in #[main] logic,
        which we’ll have at least two of: userland and designer)
    then parse PaxDesigner to manifest alongside parsing the userland component tree
    keep two definition_to_instance_traversers (each of which surfaces a get_main_component)
        the <PaxFrame> component just traverses this singular boundary (register-ed) for now; can make extensible later with different cartridges
    the root component for the engine should be PaxDesigner; the inner component is the userland component


**Dev harness:**

*TL;DR examples should just work the same as `designer-project`*    

Would be nice to have a dev harness mechanism for pax-designer through this flow.  I.e. similar to
the designer-project flow we have today.  We seem to lose this because we're retiring designer-project

In fact, given the current setup with relative paths inside the monorepo, we might get this for
free with all existing examples.  

They use a relative path for pax-engine, which uses a relative path for pax-designer,
which will thus recompile and update as we libdev.
















