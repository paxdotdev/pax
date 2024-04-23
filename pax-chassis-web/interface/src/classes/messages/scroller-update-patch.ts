export class ScrollerUpdatePatch {
    public id_chain?: number[];
    public size_x?: number;
    public size_y?: number;
    public size_inner_pane_x? : number;
    public size_inner_pane_y? : number;
    public transform? : number[];
    public scroll_x? : boolean;
    public scroll_y? : boolean;
    public subtreeDepth?: number;

    fromPatch(jsonMessage: any) {
        this.id_chain = jsonMessage["id_chain"];
        this.size_x = jsonMessage["size_x"];
        this.size_y = jsonMessage["size_y"];
        this.size_inner_pane_x = jsonMessage["size_inner_pane_x"];
        this.size_inner_pane_y = jsonMessage["size_inner_pane_y"];
        this.transform = jsonMessage["transform"];
        this.scroll_x = jsonMessage["scroll_x"];
        this.scroll_y = jsonMessage["scroll_y"];
    }

    cleanUp(){
        this.id_chain = [];
        this.size_x = 0;
        this.size_y = 0;
        this.size_inner_pane_x = 0;
        this.size_inner_pane_y = 0;
        this.transform = [];
        this.scroll_x = false;
        this.scroll_y = false;
        this.subtreeDepth = 0;
    }
}
