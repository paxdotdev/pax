
### 2024-08 - Initializing pax-designer

[x] Merge / rebase expression interpreting
# back up branch
git checkout -b zb/designer-bak-00

    migrating paxcorp/pax-designer to pax/pax-designer, including range of commits 561666d08256e52f967a6d59a3948175181cbf71 to 7767d5d4396ac8023cc456d6dda63151e076cd04 (inclusive)
    migrating paxcorp/pax-design-server to pax/pax-design-server, including range of commits 02ae043657ec1bf6053af0ad2de98041f20f6bee to 137ea86a1e27bbc84530c670f53291d784c80047 (inclusive)
    
    cd /Users/zack/code/paxcorp
    git format-patch 561666d08256e52f967a6d59a3948175181cbf71~1..HEAD --stdout -- pax-designer > pax-designer.patch
    git format-patch 02ae043657ec1bf6053af0ad2de98041f20f6bee~1..HEAD --stdout -- pax-design-server > pax-design-server.patch
    
    Apply patches: [see Solution]
    
    [error, patch does not apply]
    next to try: extract my local commits to patch; apply in the other direction on latest master
    
    [Solution] 
    git apply --3way ../pax-designer.patch
    git apply --3way ../pax-design-server.patch
    
    [ ] Update init logic, two manifests, two designtimes on other side of serialization
    
    git checkout main  # or the branch where pax-designtime exists
    git subtree split -P pax-designtime -b temp-pax-designtime


[ ] Init logic
    [x] Macro init logic (compiletime)
    [ ] Compiler-side: try-deserialize the tuple vs. the single manifest [probably make it a vec!  more extensible]
    [ ] Engine init logic (runtime)
        [ ] If in designtime build, render the root component via the designer; register the userland component for iframe  

if we’re in main and designtime
(AND this is not PaxDesigner itself — note that this code will be included in #[main] logic,
which we’ll have at least two of: userland and designer)
then parse PaxDesigner to manifest alongside parsing the userland component tree
deserialize, then
initialize a definition_to_instance_traverser from each manifest (each of which surfaces a get_main_component)
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

sed -e 's|^--- a/pax-design-server/|--- src/design_server/|' -e 's|^+++ b/pax-design-server/|+++ src/design_server/|' ../../pax-design-server.patch | git apply --3way

bookmark:

commit abe247a528e87a569f1cd5dd6333c9eb61ba8339
Author: Zack Brown <zack@pax.dev>
Date:   Mon Aug 12 15:45:14 2024 +0700

    draft of macro init logic













