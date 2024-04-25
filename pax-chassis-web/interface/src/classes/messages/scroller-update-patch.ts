export class ScrollerUpdatePatch {
    public id?: number;
    public sizeX?: number;
    public sizeY?: number;
    public sizeInnerPaneX? : number;
    public sizeInnerPaneY? : number;
    public transform? : number[];
    public scrollX? : number;
    public scrollY? : number;

    fromPatch(jsonMessage: any) {
        this.id = jsonMessage["id"];
        this.sizeX = jsonMessage["size_x"];
        this.sizeY = jsonMessage["size_y"];
        this.sizeInnerPaneX = jsonMessage["size_inner_pane_x"];
        this.sizeInnerPaneY = jsonMessage["size_inner_pane_y"];
        this.transform = jsonMessage["transform"];
        this.scrollX = jsonMessage["scroll_x"];
        this.scrollY = jsonMessage["scroll_y"];
    }

    cleanUp(){
        this.id = undefined;
        this.sizeX = 0;
        this.sizeY = 0;
        this.sizeInnerPaneX = 0;
        this.sizeInnerPaneY = 0;
        this.transform = [];
        this.scrollX = 0;
        this.scrollY = 0;
    }
}
