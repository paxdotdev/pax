
### 2024-08-09 - Initializing pax-designer

**Broad flow**

[ ] Merge / rebase expression interpreting

# back up branch
git checkout -b zb/designer-bak-00

migrating paxcorp/pax-designer to pax/pax-designer, including range of commits 561666d08256e52f967a6d59a3948175181cbf71 to 7767d5d4396ac8023cc456d6dda63151e076cd04 (inclusive)

```
git subtree split -P pax-designer -b temp-pax-designer-aug12

# Create a patch of new changes
git format-patch -k --stdout --full-index --binary origin/master..temp-pax-designer-aug12 > /tmp/new-pax-designtime-changes.patch

# Step 2: In the new repository
cd path/to/new/repo
git checkout main  # or the branch where pax-designtime now lives

# Create a temporary branch
git checkout -b temp-merge-branch

# Apply the patch
git am -k --directory=pax-designtime < /tmp/new-pax-designtime-changes.patch
```

    migrating paxcorp/pax-design-server to pax/pax-design-server, including range of commits 02ae043657ec1bf6053af0ad2de98041f20f6bee to 137ea86a1e27bbc84530c670f53291d784c80047 (inclusive)
[ ] Update init logic, two manifests, two designtimes on other side of serialization

git checkout main  # or the branch where pax-designtime exists
git subtree split -P pax-designtime -b temp-pax-designtime

# Create a patch of new changes
git format-patch -k --stdout --full-index --binary origin/main..temp-pax-designtime > /tmp/new-pax-designtime-changes.patch

# Step 2: In the new repository
cd path/to/new/repo
git checkout main  # or the branch where pax-designtime now lives

# Create a temporary branch
git checkout -b temp-merge-branch

# Apply the patch
git am -k --directory=pax-designtime < /tmp/new-pax-designtime-changes.patch


 
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
















