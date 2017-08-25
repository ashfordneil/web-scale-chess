module Main exposing (Model, Msg, update, view, subscriptions, init)

import Html exposing (..)
import Html.Attributes exposing (src, style)
import Html.Events exposing (onClick)
import Css exposing (asPairs, backgroundColor, minHeight, minWidth, px)
import Css.Colors exposing (aqua, gray, silver)


main : Program Never Model Msg
main =
    Html.program
        { init = init
        , view = view
        , update = update
        , subscriptions = subscriptions
        }



-- model


type PieceColour
    = White
    | Black


type PieceType
    = King
    | Queen
    | Bishop
    | Knight
    | Rook
    | Pawn


type alias Piece =
    { kind : PieceType
    , colour : PieceColour
    }


type ClickState
    = Unselected
    | Selected Int Int
    | Done


type alias Model =
    { board : List (List (Maybe Piece))
    , self : PieceColour
    , turn : PieceColour
    , clickState : ClickState
    }



-- update and messages


type Msg
    = Click Int Int
    | Unclick


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        Click x y ->
            (case model.clickState of
                Unselected ->
                    ( { model | clickState = Selected x y }, Cmd.none )

                _ ->
                    ( model, Cmd.none )
            )

        Unclick ->
            ( { model | clickState = Unselected }, Cmd.none )



-- view


styles : List Css.Style -> Attribute msg
styles =
    asPairs >> style


renderPiece : Piece -> String
renderPiece piece =
    case piece.kind of
        King ->
            (case piece.colour of
                Black ->
                    "https://upload.wikimedia.org/wikipedia/commons/e/e3/Chess_kdt60.png"

                White ->
                    "https://upload.wikimedia.org/wikipedia/commons/3/3b/Chess_klt60.png"
            )

        Queen ->
            (case piece.colour of
                Black ->
                    "https://upload.wikimedia.org/wikipedia/commons/a/af/Chess_qdt60.png"

                White ->
                    "https://upload.wikimedia.org/wikipedia/commons/4/49/Chess_qlt60.png"
            )

        Bishop ->
            (case piece.colour of
                Black ->
                    "https://upload.wikimedia.org/wikipedia/commons/8/81/Chess_bdt60.png"

                White ->
                    "https://upload.wikimedia.org/wikipedia/commons/9/9b/Chess_blt60.png"
            )

        Knight ->
            (case piece.colour of
                Black ->
                    "https://upload.wikimedia.org/wikipedia/commons/f/f1/Chess_ndt60.png"

                White ->
                    "https://upload.wikimedia.org/wikipedia/commons/2/28/Chess_nlt60.png"
            )

        Rook ->
            (case piece.colour of
                Black ->
                    "https://upload.wikimedia.org/wikipedia/commons/a/a0/Chess_rdt60.png"

                White ->
                    "https://upload.wikimedia.org/wikipedia/commons/5/5c/Chess_rlt60.png"
            )

        Pawn ->
            (case piece.colour of
                Black ->
                    "https://upload.wikimedia.org/wikipedia/commons/c/cd/Chess_pdt60.png"

                White ->
                    "https://upload.wikimedia.org/wikipedia/commons/0/04/Chess_plt60.png"
            )


renderSquare : Int -> Int -> Bool -> Maybe Piece -> Html Msg
renderSquare x y highlighted elem =
    let
        base =
            if highlighted then
                aqua
            else if (x + y) % 2 == 1 then
                gray
            else
                silver

        piece =
            Maybe.withDefault [] (Maybe.map (renderPiece >> src >> List.singleton) elem)

        event =
            if highlighted then
                Unclick
            else
                (Click x y)
    in
        td []
            [ button
                [ onClick event
                , styles
                    [ backgroundColor base
                    , minHeight (px 120)
                    , minWidth (px 120)
                    ]
                ]
                [ img piece [] ]
            ]


renderBoard : Int -> Int -> List (List (Maybe Piece)) -> Html Msg
renderBoard selectedX selectedY board =
    table []
        (board
            |> List.indexedMap
                (\y row ->
                    row
                        |> List.indexedMap (\x piece -> renderSquare x y (x == selectedX && y == selectedY) piece)
                        |> tr []
                )
        )


view : Model -> Html Msg
view model =
    let
        ( x, y ) =
            case model.clickState of
                Unselected ->
                    ( -1, -1 )

                Done ->
                    ( -1, -1 )

                Selected x y ->
                    ( x, y )
    in
        case model.self of
            White ->
                renderBoard x y model.board

            Black ->
                renderBoard x y (model.board |> List.reverse |> List.map List.reverse)



-- subscriptions


subscriptions : Model -> Sub Msg
subscriptions model =
    Sub.none



-- init


initBoard : List (List (Maybe Piece))
initBoard =
    List.concat
        [ [ List.map (\x -> Just ({ kind = x, colour = Black }))
                [ Rook
                , Knight
                , Bishop
                , Queen
                , King
                , Bishop
                , Knight
                , Rook
                ]
          , List.repeat 8 (Just ({ kind = Pawn, colour = Black }))
          ]
        , List.repeat 4 (List.repeat 8 Nothing)
        , [ List.repeat 8 (Just ({ kind = Pawn, colour = White }))
          , List.map (\x -> Just ({ kind = x, colour = White }))
                [ Rook
                , Knight
                , Bishop
                , Queen
                , King
                , Bishop
                , Knight
                , Rook
                ]
          ]
        ]


init : ( Model, Cmd Msg )
init =
    ( ({ board = initBoard, self = White, turn = White, clickState = Unselected }), Cmd.none )
