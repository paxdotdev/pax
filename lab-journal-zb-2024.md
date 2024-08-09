


### 2024-08-09 - Initializing pax-designer

**Broad flow**

if we’re in main and designtime
    (AND this is not PaxDesigner itself — note that this code will be included in #[main] logic,
        which we’ll have at least two of: userland and designer)
    then parse PaxDesigner to manifest along with parsing the userland component tree
    keep two definition_to_instance_traversers (each of which surfaces a get_main_component)
        the <PaxFrame> component just traverses this singular boundary (register-ed) for now; can make extensible later
    the root component for the engine should be PaxDesigner; the inner component is the userland component

**Dev harness:**  
    
Would be nice to have a dev harness mechanism for pax-designer through this flow.  I.e. similar to
the designer-project flow we have today.  Since we're retiring designer-project, one solution is to
support something like `--libdev[="../../../pax-designer"]` as a CLI param, which looks to the provided path for the source 
code of the pax-designer directory




